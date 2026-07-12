
# A. Data Validation

- Validate at every trust boundary — client-side for UX, server-side for security, and again at the domain layer for business rules (e.g. `volume > 0`);
- Never trust client validation alone
- Return structured error responses with field-level details (e.g. `{"field":"volume","error":"must be positive"}`)
- Keep validation close to the entry point so invalid data never reaches business logic.

examples:  
- Reject `volume=0` or `price=0` in `BuyRequest` before reaching `buy_impl`
- Return structured JSON errors instead of plain text `400`
- Validate `username` is non-empty in `AllocationQuery`
- Add `SellRequest` volume > 0 check

# B. Error Handling

### 1. Return structured error responses with consistent shape

```rust
#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: String,
    field: Option<&'static str>,
}

async fn sell(
    // ...
    req: web::Json<SellRequest>
) -> impl Responder {
    if req.volume == 0 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "sell_validation_failed",
            message: "volume must be greater than 0".to_string(),
            field: Some("volume"),
        });
    }
```     

### 2. Error classification: client errors (4xx) vs server errors (5xx)

**4xx — Client Errors:**
- 400 — invalid input
- 401 — not authenticated
- 403 — authenticated but not authorized
- 404 — resource not found
- 408 — request timeout (client too slow)
- 409 — conflict (duplicate)
- 422 — valid JSON but fails business rules
- 429 — rate limited

**5xx — Server Errors:**
- 500 — unexpected server error
- 502 — bad gateway (upstream returned invalid response)
- 503 — service unavailable (downstream down, circuit open)
- 504 — gateway timeout (upstream too slow)


### 3. Never expose internal details (stack traces, DB errors) to clients


### 4. Idempotency — retrying a failed request shouldn't cause duplicate side effects

e.g. If a `/buy` request times out and the client retries, the server might process it twice — idempotency means the second request has no additional effect, typically implemented by the client sending a unique `idempotency-key` header and the server storing processed request IDs to detect and deduplicate retries.

**Client sends:**
```bash
curl -X POST http://localhost:8080/buy \
  -H "Idempotency-Key: req-42" \
  -H "Content-Type: application/json" \
  -d '{"user":"u1","volume":10,"price":3}'
```

**Server stores processed keys:**
```rust
#[derive(Default)]
pub struct AppState {
    // Idempotency key → response body
    processed_keys: DashMap<String, String>, 
    // ...
}

async fn buy(
    state: web::Data<AppState>,
    req_http: HttpRequest,
    req: web::Json<BuyRequest>,
) -> impl Responder {
    if let Some(key) = req_http.headers() // e.g. key: "req-42"
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(cached) = state.processed_keys.get(key) {
            return HttpResponse::Ok().body(cached.clone()); 
                // return cached HTTP response
        }
        let response = buy_impl(&state, req.0);
        state.processed_keys.insert(key.to_string(), response.clone());
        return HttpResponse::Ok().body(response);
    }
    HttpResponse::BadRequest().body("Idempotency-Key header required")
}
```

### 5. Circuit breaker pattern — stop calling a failing downstream service  

Three states: 
- closed (normal, requests pass through)
- open (too many failures, requests blocked immediately)
- half-open (after a timeout, let one request through to test if service recovered).

e.g.  
```
//// PriceFeed — circuit breaker for downstream price feed calls
////
//// Fetches BTC-USD spot price from Coinbase public API.
//// Tracks failures and opens the circuit after 3 consecutive failures,
//// blocking further requests for 10 secs before retrying (half-open).
////
//// States:
////   Closed   — normal, requests pass through
////   Open     — too many failures, requests blocked immediately
////   Half-open — 10s elapsed, one request allowed through to test recovery
////
```

See impl. in:  
[`tw_err_circuit_breaker.rs`](tw_err_circuit_breaker.rs)



### 6. Graceful degradation — partial failure shouldn't bring down the whole system

e.g. If `get_btc_price()` fails, the `/buy` endpoint should still work — just skip price validation or use a cached/default price instead of returning 500 and blocking all buys.  


### 7. Correlation IDs — trace a request across services via a shared ID in logs and responses

Every incoming request gets a unique ID (UUID), passed through all log lines and downstream calls so we can grep one ID and see the full request journey across services.
```
# 1. A buy request arrives
INFO  correlation_id=abc-123 method=POST path=/buy user=u1 volume=10 price=3

# 2. JWT decoded
INFO  correlation_id=abc-123 has_feature_new_alloc=true

# 3. Price feed called
INFO  correlation_id=abc-123 calling=coinbase url=BTC-USD/spot

# 4. Price feed responded
INFO  correlation_id=abc-123 btc_price=108432.50 latency_ms=45

# 5. buy_impl executed
INFO  correlation_id=abc-123 buy_impl: close time.busy=38µs time.idle=12µs

# 6. Response sent
INFO  correlation_id=abc-123 status=200 total_latency_ms=47
```

Now if a client complains "my buy at 10:23 failed", we grep `abc-123` and see exactly where it broke.

e.g.  
```rust
// middleware generates ID if not present
let correlation_id = req_http
    .headers()
    .get("X-Correlation-ID")
    .and_then(|v| v.to_str().ok())
    .unwrap_or(&uuid::Uuid::new_v4().to_string());

// log it
tracing::info!(correlation_id, "buy request received");

// return it in response header so client can reference it
HttpResponse::Ok()
    .insert_header(("X-Correlation-ID", correlation_id))
    .json(...)
```

# C. Specific challenge and how to solve?

The JWT `MissingRequiredClaim("exp")` error was a real validation gap,
`decode_jwt()` silently returned `Err` and `has_feature_new_alloc` flag in buy handler fell back to `false` although it was enabled in token, meaning the feature flag failed invisibly with no error returned to the client and no log indicating why — the fix was disabling `exp` validation and adding `let _ = dbg!(decode_jwt(token))` to surface the silent failure.


``` 
curl -s -X POST http://localhost:8080/buy -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZmVhdHVyZXMiOlsibmV3X2FsbG9jYXRpb24iXX0.4Fb63XPR1uY_yUjULL0IifKP5SmkIb1qYvLMyTxRMfk" -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"  

// server:

[src\tw_main.rs:356:13] decode_jwt(token.unwrap()) = Ok(
    Claims {
        sub: "1234567890",
        features: [
            "new_allocation",
        ],
    },
)
[src\tw_main.rs:363:5] has_feature_new_allocation = true

``` 

for more about this see 
[`tw_api_versioning.md`](tw_api_versioning.md)  
