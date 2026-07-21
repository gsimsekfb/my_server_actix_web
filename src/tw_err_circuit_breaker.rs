use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::Deserialize;

//// PriceFeed — circuit breaker pattern for upstream price feed calls
//// See fn get_btc_price for more info

// curl -s https://api.coinbase.com/v2/prices/BTC-USD/spot
// {"data":{"amount":"64288.32","base":"BTC","currency":"USD"}}
// Two stuuct levels of nesting
#[derive(Deserialize)]
struct CoinbaseResponse { data: PriceData, }

#[derive(Deserialize)]
struct PriceData { amount: String, }

#[derive(Debug, Default, PartialEq)]
enum CircuitState { 
    #[default] Closed, 
    Open(Instant),
    HalfOpen
}

#[derive(Debug, PartialEq)]
pub enum PriceFeedError {
    CircuitOpen,
    FetchFailed,
    ParseFailed,
}

impl PriceFeedError {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriceFeedError::CircuitOpen => "circuit_open",
            PriceFeedError::FetchFailed => "fetch_failed",
            PriceFeedError::ParseFailed => "parse_failed",
        }
    }
}

#[derive(Debug)]
pub struct PriceFeed {
    failures: AtomicU64,
    circuit_state: std::sync::Mutex<CircuitState>,
    url: String
}

impl Default for PriceFeed {
    fn default() -> Self {
        Self::new("https://api.coinbase.com/v2/prices/BTC-USD/spot")
    }
}

impl PriceFeed {

    fn new(url: impl Into<String>) -> Self {
        Self { 
            failures: AtomicU64::new(0),
            circuit_state: std::sync::Mutex::new(CircuitState::Closed),
            url: url.into() 
        }
    }

    /// Fetches BTC-USD spot price from Coinbase public API.
    /// Tracks failures and opens the circuit after 3 consecutive failures,
    /// blocking further requests for 10 seconds before retrying (half-open).
    ///
    /// States:
    ///   Closed   — normal, requests pass through
    ///   Open     — too many failures, requests blocked immediately. After 
    ///              10s, only one request allowed through to test recovery
    ///   Half-open — concurrent callers during half-open get CircuitOpen 
    ///               immediately
    ///
    /// Usage: PriceFeed::new() returns Arc<PriceFeed>; 
    ///        call get_btc_price().wait
    /// Also see tw_main.rs -> btc-price handler.
    pub async fn get_btc_price(&self) -> Result<f64, PriceFeedError> {
        // close circuit to try again if 10 secs past after last req. to open 
        // circuit
        {
            let mut circuit_state = self.circuit_state.lock().unwrap();
            match *circuit_state {
                CircuitState::Open(since) => {
                    if since.elapsed() < Duration::from_secs(10) {
                        return Err(PriceFeedError::CircuitOpen); // block request
                    }
                    // only first caller gets through to call upstream
                    *circuit_state = CircuitState::HalfOpen;
                }
                CircuitState::HalfOpen => {
                    // other threads still blocked
                    return Err(PriceFeedError::CircuitOpen) 
                }
                CircuitState::Closed => {} // do nothing
            }
        }

        // call upstream
        // todo: reqwest::get(...) builds a throwaway client on every call 
        // instead of reusing a pooled reqwest::Client stored on PriceFeed — 
        // so, no connection reuse, extra TLS handshakes under load.
        let result = reqwest::get(&self.url)
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| PriceFeedError::FetchFailed);
            // todo: add timeout

        match result {
            Ok(resp) => {
                // todo: if the single shot caller from Open match case, get 
                // this parse fail, the circuit will be stuck on HalfOpen state
                let resp_price: CoinbaseResponse = 
                    resp.json().await.map_err(|_| PriceFeedError::ParseFailed)?;
                self.failures.store(0, Ordering::Relaxed); // reset on success
                let mut circuit_state = self.circuit_state.lock().unwrap();
                if *circuit_state != CircuitState::Closed {
                    *circuit_state = CircuitState::Closed;
                }
                Ok(resp_price.data.amount.parse()
                    .map_err(|_| PriceFeedError::ParseFailed)?
                )
            }
            Err(e) => {
                let failures = 
                    self.failures.fetch_add(1, Ordering::Relaxed) + 1;
                let mut circuit_state = self.circuit_state.lock().unwrap();
                if failures >= 3 || *circuit_state == CircuitState::HalfOpen {
                    *circuit_state = CircuitState::Open(Instant::now());
                }
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_1_fetch_price_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("GET", "/v2/prices/BTC-USD/spot")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"amount":"64288.32","base":"BTC","currency":"USD"}}"#)
            .create_async().await;

        let feed = PriceFeed::new(
            format!("{}/v2/prices/BTC-USD/spot", server.url())
        );
        let price = feed.get_btc_price().await.unwrap();
        assert_eq!(price, 64288.32);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_2_circuit_opens_after_3_failures() {
        let feed = PriceFeed::new("WRONG_URL");

        // First 3 tries should fail
        for _ in 1..=3 {
            let price = feed.get_btc_price().await;
            assert_eq!(price, Err(PriceFeedError::FetchFailed));            
        }
        // 4th try should open circuit
        let price = feed.get_btc_price().await;
        assert_eq!(price, Err(PriceFeedError::CircuitOpen));
    }

    #[tokio::test]
    async fn test_3_open_circuit_blocks_requests_without_upstream_call() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("GET", "/v2/prices/BTC-USD/spot")
            .expect(0) // no HTTP request made
            .create_async().await;

        let feed = PriceFeed::new(
            format!("{}/v2/prices/BTC-USD/spot", server.url())
        );
        *feed.circuit_state.lock().unwrap() = CircuitState::Open(Instant::now());
        for _ in 0..3 {
            let result = feed.get_btc_price().await;
            assert!(matches!(result, Err(PriceFeedError::CircuitOpen)));
        }

        // verify the number of requests made, specified by .expect(n) above
        mock.assert_async().await;
    }

    /// Note: can be flaky in the unlikely case where 1 task actually succeeds
    /// to complete a fetch which will close the circuit and increase the number
    /// of requests tp the mock server. Barrier is to prevent it but does not
    /// entirely eliminate flaky case.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        // Make it truly parallel, #[tokio::test] without flavor args spins 
        // up a single-threaded runtime
    async fn test_4_half_open_circuit_trial_succeeds() {
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("GET", "/v2/prices/BTC-USD/spot")
            .expect(2)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":{"amount":"64288.32","base":"BTC","currency":"USD"}}"#)
            .create_async().await;

        let feed = Arc::new(PriceFeed::new(
            format!("{}/v2/prices/BTC-USD/spot", server.url())
        ));
        *feed.circuit_state.lock().unwrap() = 
            CircuitState::Open(Instant::now() - Duration::from_secs(11));
            // simulate 11 secs wait

        // At least 10 secs wait to switch to HalfOpen state
        println!("\n-- simulate 11 secs wait...");

        // state: HalfOpen

        // Concurrent requests hitting HalfOpen simultaneously
        let mut join_set = tokio::task::JoinSet::new(); // set of 100 tasks
        let barrier = Arc::new(tokio::sync::Barrier::new(100));
            // barrier: all 100 tasks reach get_btc_price() at genuinely 
            // the same instant, rather than trusting JoinSet::spawn ordering
        for _ in 0..100 {
            let feed_clone = feed.clone();
            let barrier_clone = barrier.clone();
            join_set.spawn(async move {
                barrier_clone.wait().await;
                let _ = feed_clone.get_btc_price().await;
            });
        }
        join_set.join_all().await;

        // 2nd call to mock server
        let result = feed.get_btc_price().await;
        assert_eq!(result, Ok(64288.32));

        // verify the number of requests made, specified by .expect(n) above
        mock.assert_async().await;
    }

    // todo: this should be done with concurrent requests like in test_4
    #[tokio::test]
    async fn test_5_half_open_circuit_trial_fails_and_circuit_reopens() {
        let feed = PriceFeed::new("/v2/prices/BTC-USD/spot");
        *feed.circuit_state.lock().unwrap() = 
            CircuitState::Open(Instant::now() - Duration::from_secs(11));
            // simulate 11 secs wait

        // At least 10 secs wait to switch to HalfOpen state
        println!("\n-- simulate 11 secs wait...");

        // state: HalfOpen

        // Only one trial request is allowed to call upstream 
        let result = feed.get_btc_price().await.unwrap_err();
        assert_eq!(result, PriceFeedError::FetchFailed);
        // circuit re-opens
        let result = feed.get_btc_price().await.unwrap_err();
        assert_eq!(result, PriceFeedError::CircuitOpen);
    }

}
