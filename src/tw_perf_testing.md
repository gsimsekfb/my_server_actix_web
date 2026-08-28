
## Performance Tests For Production

### Production Performance Pyramid
| Test Type | Tool | Focus | When to Run |
| :--- | :--- | :--- | :--- |
| **Micro-bench** | **Criterion** | `buy_impl` algorithmic efficiency | Every PR |
| **Load Test** | **Drill / wrk** | Lock contention & RPS | Release candidates |
| **Profiling** | **Flamegraph** | CPU "Hot paths" | During optimization |
| **Soak Test** | **Drill / wrk** | Memory leaks & resource exhaustion | Pre-release (hours) |

## Tests
[1. Micro-benchmarking](#1--micro-benchmarking---the-algorithm-level)  
[2. HTTP Load Testing](#2--http-load-testing---the-system-level)  
[3. Continuous Profiling](#3--continuous-profiling---the-visibility-level)  
[4. Soak / Endurance Testing](#4--soak-aka-endurance-testing---the-longevity-level)  


----------------------------------------------------------------------------


### 1- Micro-benchmarking - The Algorithm Level
Since we are optimizing specific operations like `retain` vs `pop_first` on a `BTreeMap`, we should use **Criterion.rs**. This is the industry standard for measuring small units of code with statistical rigor.
*   **The Goal:** Isolate the `buy_impl` logic from all network/locking overhead to see how it scales with the number of orders.
*   **Setup:** Create benchmarks that populate a map with $10^3, 10^4, \text{and } 10^5$ bids to measure the exact nanosecond cost of rebalancing.

#### Implementation:

Note: The following is for "the first version" of `sell_impl`, also see the bench fn for `buy_impl` in the same file.

A sample **Criterion** benchmark `benches/allocation.rs` measures how long `sell_impl` takes to allocate all outstanding bids as the number of bids scales from 1K to 100K.

Run it with:
```rust
cargo bench --bench allocation -- sell_impl
```

```rust
Gnuplot not found, using plotters backend
Benchmarking sell_impl/retain/1000: Warming up for 3.0000 s
                                           // median
sell_impl/retain/1000   time:   [1.1392 ms 1.1933 ms 1.2575 ms]
                        // p — statistical significance;
                        // p < 0.05 means the change is real, not random noise
                        // change - vs last cargo bench
                        change: [-38.435% -34.694% -31.078%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  4 (4.00%) high mild
    // Outliers — samples that deviate significantly from the median; 
    // "mild" = slightly off, "severe" = very off, 
    // often caused by OS scheduling or GC

sell_impl/retain/10000  time:   [11.681 ms 12.267 ms 12.946 ms]
                        change: [-44.856% -39.987% -34.194%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 10 outliers among 100 measurements (10.00%)
  6 (6.00%) high mild
  4 (4.00%) high severe
Benchmarking sell_impl/retain/100000: Warming up for 3.0000 s


sell_impl/retain/100000 time:   [132.57 ms 136.94 ms 141.61 ms]
                        change: [-21.288% -17.064% -12.507%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 9 outliers among 100 measurements (9.00%)
  8 (8.00%) high mild
  1 (1.00%) high severe

```

#### Bench Result:

Linear scaling — roughly O(n):

| Bids | Time |
|------|------|
| 1K | ~1.2ms |
| 10K | ~12ms |
| 100K | ~137ms |

10x more bids → ~10x more time, confirms O(n) behavior of `retain`. But for 100K+ concurrent open bids we might want to bench with `pop_first` with early exit.

[↑ Back to top](#tests)

------------------

### 2- HTTP Load Testing - The System Level
Once the algorithm is fast, we must test the **lock contention** in Axum handlers under high load. Use a tool like **Drill** or **Goku**, which are written in Rust for high-throughput benchmarking.
*   **The Goal:** Measure how many Requests Per Second (RPS) `buy` handler can process before the fine-grained locks cause latency spikes (P99s).
*   **Metric to Watch:** **Tail Latency.** In financial systems, the average latency is often a "lie"; we care about the P99 or P99.9—the worst-case delay experienced by users.
  

**What to look at in the output:**
```
Requests/sec:   12345.67       ← throughput
Latency distribution:
  50% in 0.8ms                 ← median
  99% in 4.2ms                 ← tail latency (the important one)
```

**You've found something interesting when:**
- p99 is **10x+ higher** than p50 → contention somewhere
- Throughput **plateaus or drops** as you add connections → you've hit the ceiling
- Any **errors appear** → something is breaking under load


#### The test
``` 
# 10 concurrent connections, 10 secs, unlimited requests to find the throughput ceiling
wsl$ 
hey -c 10 -z 10s -m POST \
  -d '{"user":"u1","volume":10,"price":3}' \
  -H "Content-Type: application/json" \
  http://localhost:8080/buy

// same cmd, one line
hey -c 10 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy


Summary:
  Total:        10.0078 secs
  Slowest:      0.0293 secs
  Fastest:      0.0004 secs
  Average:      0.0025 secs
  Requests/sec: 4006.1798  // throughput

  Total data:   12177166 bytes
  Size/request: 303 bytes

Response time histogram:
  0.000 [1]     |
  0.003 [30240] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.006 [8505]  |■■■■■■■■■■■
  0.009 [993]   |■
  0.012 [197]   |
  0.015 [108]   |
  0.018 [19]    |
  0.021 [8]     |
  0.024 [13]    |
  0.026 [5]     |
  0.029 [4]     |

Latency distribution:
  10% in 0.0009 secs
  25% in 0.0010 secs
  50% in 0.0023 secs  // median
  75% in 0.0033 secs
  90% in 0.0045 secs
  95% in 0.0055 secs
  99% in 0.0088 secs  // aka tail latency - most important
    // P99: 99% of requests complete in 0.088 secs or less 

Details (average, fastest, slowest):
  DNS+dialup:   0.0000 secs, 0.0004 secs, 0.0293 secs
  DNS-lookup:   0.0000 secs, 0.0000 secs, 0.0018 secs
  req write:    0.0000 secs, 0.0000 secs, 0.0030 secs
  resp wait:    0.0024 secs, 0.0003 secs, 0.0292 secs
  resp read:    0.0001 secs, 0.0000 secs, 0.0104 secs

Status code distribution:
  [200] 40093 responses
``` 


**What to look at in the output:**
```
Requests/sec:   12345.67       ← throughput
Latency distribution:
  50% in 0.8ms                 ← median
  99% in 4.2ms                 ← tail latency (the important one)
```

**You've found something interesting when:**
- p99 is **10x+ higher** than p50 → contention somewhere
- Throughput **plateaus or drops** as you add connections → you've hit the ceiling
- Any **errors appear** → something is breaking under load

That's your V1. Everything else (constant-rate testing, realistic payloads, sustained soak tests) builds on top of this baseline.


#### Next:
Run the **same endpoint** at increasing concurrency levels (e.g., 1 → 10 → 50 → 100 → 500) and record **requests/sec** and **p99 latency** at each level. That's it.

```bash
# 10 concurrent connections, 10 secs, unlimited requests (already done above)
hey -c 10 -z 10s http://localhost:8080/endpoint

# 100 concurrent connections, 10 secs, unlimited requests
hey -c 100 -z 10s http://localhost:8080/endpoint

# 500 concurrent connections, 10 secs, unlimited requests
hey -c 500 -z 10s http://localhost:8080/endpoint
```

[↑ Back to top](#tests)

-----


### 3- Continuous Profiling - The Visibility Level
In production, we cannot always reproduce performance issues locally. Use **Flamegraphs** (via `cargo-flamegraph`) to visualize exactly where CPU time is being spent—whether it's inside the `BTreeMap` search or waiting for a `Mutex`.
*   **The Goal:** Identify "hot paths" and locking bottlenecks visually.
*   **Tooling:** Use the **Tracing** crate to instrument code. This allows we to collect timing data across `buy` and `buy_impl` boundary without stopping the service.

**A. flamegraph** — run under load, generate flamegraph to see where CPU time goes:  
**B. tracing spans** — add one span per handler to measure buy/sell time:

These two together cover the most important visibility — flamegraph shows *where* time is spent, tracing shows *how long* each operation takes.  

-----

The minimal high-value approach tests in practice:

**A. flamegraph** — run under load, generate flamegraph to see where CPU time goes:
```bash
cargo flamegraph --bin twn -- &
hey -c 50 -z 10s ... 
# flamegraph.svg generated automatically
```

#### The test:

Using samply instead of flamegraph
```bash
# 1. Install
cargo install samply

# 2. Build with debug symbols
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --bin twn

# 3. Profile under load
samply record ./target/release/twn
# 10 concurrent connections, 10 secs, unlimited requests
hey -c 10 -z 10s -m POST \
  -d '{"user":"u1","volume":10,"price":3}' \
  -H "Content-Type: application/json" \
  http://localhost:8080/buy

# 4. Open localhost:3000
samply load profile.json.gz
Local server listening at http://127.0.0.1:3000
```

**B. tracing spans** — add one span per handler to measure buy/sell time:
```rust
use tracing::instrument;

#[instrument]
fn buy_impl(...) { ... }
```

**The test:**

i. add `#[instrument]` for `fn buy` and `fn buy_impl`

ii. enable this line in main.rs
```rust    
// For profiling w/ tracing spans - see tw\_perf\_testing.md for more
tracing\_subscriber::fmt()
    .with\_env\_filter(
        tracing\_subscriber::EnvFilter::from\_default\_env()
            .add\_directive("info".parse().unwrap())
    )
    .with\_span\_events(tracing\_subscriber::fmt::format::FmtSpan::CLOSE)
    .init();

```

iii. 
```
cargo r --release --bin twn

# send buy request
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"

2026-06-13T10:08:19  INFO buy{req=Json(BuyRequest { user: "u1", volume: 100, price: 3 })}:buy_impl{buy_req=BuyRequest { user: "u1", volume: 100, price: 3 }}: twn: close time.busy=38.9µs time.idle=31.7µs

2026-06-13T10:08:19  INFO buy{req=Json(BuyRequest { user: "u1", volume: 100, price: 3 })}: twn: close time.busy=476µs time.idle=33.3µs
```

Read the logs — look for `close` lines with `time.busy` and `time.idle` fields

`time.busy` = actual CPU work  
`time.idle` = time waiting (I/O, locks, awaits).


**Test result:**   
`buy_impl` takes ~25-47µs of actual CPU (`time.busy`), while the full `buy` handler takes ~430-604µs — the ~550µs difference is overhead from HTTP parsing, JSON deserialization, and lock acquisition (`time.idle` + framework overhead).


[↑ Back to top](#tests)

----


### 4- Soak aka Endurance Testing - The Longevity Level
The three categories above test peak performance, but production services run for days. A soak test runs **moderate, sustained load for hours** to surface issues that only appear over time.
*   **The Goal:** Catch memory leaks, connection pool exhaustion, file-descriptor leaks, or gradual latency degradation that short burst tests miss.
*   **Setup:** Run Drill/wrk at ~60-70% of max RPS for 2-4 hours; monitor RSS memory, open FDs, and P99 latency trends over time.

#### The test
Run `hey -c 30 -z 2h ...` at ~60% of your peak RPS, while periodically checking `ps aux` for memory growth and watching P99 latency for gradual increase.

a. Prep: Find cmd for ~60% of your peak RPS

e.g. try these cmds, check `Requests/sec: <number>`:  
(or add/change `-q 2400`, keep `-c` high enough to sustain it)

```bash
hey -c 10  -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

hey -c 50  -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

hey -c 100 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

hey -c 500 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy
```


b. Test

```bash
# 1. Start server
cargo run --bin twn --release

# 2. Run sustained load (adjust -q or -c to ~60% of your peak RPS, 
#    see section a above)
hey -c 30 -q 2400 -z 2h -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

# 3. Monitor memory every 5 minutes
ps aux | grep twn

# 4. Periodically drain bids to prevent unbounded growth
curl -s -X POST -d '{"volume":999999999}' -H "Content-Type: application/json" http://localhost:8080/sell
```



[↑ Back to top](#tests)

---
