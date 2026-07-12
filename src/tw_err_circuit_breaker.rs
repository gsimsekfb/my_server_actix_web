use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::Deserialize;

//// PriceFeed — circuit breaker pattern for downstream price feed calls
//// See fn get_btc_price for more info

// curl -s https://api.coinbase.com/v2/prices/BTC-USD/spot
// {"data":{"amount":"64288.32","base":"BTC","currency":"USD"}}
// Two stuuct levels of nesting
#[derive(Deserialize)]
struct CoinbaseResponse { data: PriceData, }

#[derive(Deserialize)]
struct PriceData { amount: String, }

#[derive(Debug, Default)]
enum CircuitState { 
    #[default] Closed, 
    Open(Instant)
}

#[derive(Debug, Default)]
pub struct PriceFeed {
    failures: AtomicU64,
    state: std::sync::Mutex<CircuitState>,
}

impl PriceFeed {

    /// Fetches BTC-USD spot price from Coinbase public API.
    /// Tracks failures and opens the circuit after 3 consecutive failures,
    /// blocking further requests for 10 seconds before retrying (half-open).
    ///
    /// States:
    ///   Closed   — normal, requests pass through
    ///   Open     — too many failures, requests blocked immediately
    ///   Half-open — 10s elapsed, one request allowed through to test recovery
    ///
    /// Usage: PriceFeed::new() returns Arc<PriceFeed>; 
    ///        call get_btc_price().wait
    /// Also see tw_main.rs -> btc-price handler.
    pub async fn get_btc_price(&self) -> Result<f64, &'static str> {
        // close circuit to try again if 10 secs past after last req. to open 
        // circuit
        {
            let mut state = self.state.lock().unwrap();
            if let CircuitState::Open(since) = *state {
                if since.elapsed() < Duration::from_secs(10) {
                    return Err("circuit_open"); // block request
                }
                *state = CircuitState::Closed; // half-open: try again
                self.failures.store(0, Ordering::Relaxed);
            }
        }

        // call downstream
        const BTC_PRICE: &str = 
            "https://api.coinbase.com/v2/prices/BTC-USD/spot";
        let result = reqwest::get(BTC_PRICE) // Result<Response, &str>
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| "fetch_failed");
            // todo: add timeout

        match result {
            Ok(resp) => {
                let resp_price: CoinbaseResponse = 
                    resp.json().await.map_err(|_| "parse_failed")?;
                self.failures.store(0, Ordering::Relaxed); // reset on success
                Ok(resp_price.data.amount.parse().map_err(|_| "parse_failed")?)
            }
            Err(e) => {
                let failures = 
                    self.failures.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= 3 {
                    *self.state.lock().unwrap() = 
                        CircuitState::Open(Instant::now());
                }
                Err(e)
            }
        }
    }
}
