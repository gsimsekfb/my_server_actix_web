

## x. Add metrics for other ops e.g. sell_impl
...

## x. `src/tw_main.rs:229-233, 270-274` — metrics are on the hot path

### chatgpt:
🟠 MAJOR
1. `src/tw_main.rs:229-233, 270-274` — metrics are on the hot path
`histogram!`, `counter!`, and `gauge!` are looked up/registered on every `buy_impl()` execution.
For this project this matters because you're using the API for throughput/latency benchmarking. The instrumentation itself becomes part of the measured workload.
The `metrics` documentation explicitly notes that retaining a histogram handle can amortize registration cost.
Recommendation: initialize/retain metric handles rather than repeatedly registering them inside `buy_impl()`.
Severity: MAJOR because this can invalidate comparisons such as your previous `hey` benchmarks.

### Claude answer:

**What actually happens:** the `metrics` crate macros don't "register" on every call (registration happens once, internally, the first time a unique key is seen). What happens on every call is a **registry lookup** — `counter!(name, labels...)` builds a `Key` (hashing the name + label strings) and looks it up in the exporter's internal map to get back a handle, then calls `.increment()` on it. That lookup + `Key` construction is the real per-call cost, not re-registration.

**Is it a MAJOR severity issue for your benchmarks?** Plausible, but unverified — you haven't profiled it. Given your own `perf_perf_hotspot.md` findings (BTreeMap comparison ~66%, `#[instrument]` ~10-12%), a hashmap lookup + label-string handling per call could easily be in the same ballpark as the tracing overhead you already flagged — worth checking before trusting `hey` numbers that included this instrumentation.

**Fix — retain handles instead of macro-calling every time.** Add them to `AppState` (same pattern as `buy_seq_no`/`supply`) and initialize once:

```rust
use metrics::{Counter, Gauge, Histogram};

#[derive(Debug)]
pub struct AppState {
    // ...existing fields...
    buy_duration: Histogram,
    buy_requests: Counter,
    open_bids: Gauge,
    supply_gauge: Gauge,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            buy_seq_no: AtomicU64::new(0),
            supply: AtomicU64::new(0),
            allocations: DashMap::new(),
            bids: RwLock::new(BTreeMap::new()),
            price_feed: Arc::new(PriceFeed::default()),
            buy_duration: histogram!("buy_impl_duration_seconds"),
            buy_requests: counter!("http_requests_total", "endpoint" => "buy"),
            open_bids: gauge!("open_bids_count"),
            supply_gauge: gauge!("supply_current"),
        }
    }
}
```

Then in `buy_impl`, use the retained handles instead of macros:

```rust
pub fn buy_impl(
    buy_seq_no: &AtomicU64,
    supply: &AtomicU64,
    allocations: &DashMap<String, u64>,
    bids: &mut BTreeMap<PriceSeqPair, Bid>,
    buy_req: BuyRequest,
    metrics: &AppState, // or pass individual handles
) {
    // ...existing logic...

    metrics.buy_duration.record(start.elapsed().as_secs_f64());
    metrics.buy_requests.increment(1);
    metrics.open_bids.set(bids.len() as f64);
    metrics.supply_gauge.set(supply.load(Ordering::Relaxed) as f64);
}
```

This turns each call site from "hash + lookup + increment" into just "increment" on an already-resolved handle — closer to a bare atomic op, which is what you actually want on this path given your existing perf sensitivity.

**One caveat:** `http_requests_total` currently only has one label combination (`endpoint="buy"`), so retaining it is safe. If you later add labels with dynamic values (e.g. per-user or per-status-code), you can't pre-retain a single handle — you'd need a small `DashMap<LabelKey, Counter>` cache instead, since the label combinations aren't known upfront.

**Bottom line:** the recommendation is sound, the severity claim is unverified (needs a `perf` pass to confirm vs. assumed), and "registered on every execution" should really read "looked up on every execution" — but the fix and priority are correct.

