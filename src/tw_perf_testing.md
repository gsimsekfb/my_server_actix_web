## Performance Tests For Production

### 1. Micro-benchmarking (The "Algorithm" Level)
Since we are optimizing specific operations like `retain` vs `pop_first` on a `BTreeMap`, we should use **Criterion.rs**. This is the industry standard for measuring small units of code with statistical rigor.
*   **The Goal:** Isolate the `buy_impl` logic from all network/locking overhead to see how it scales with the number of orders.
*   **Setup:** Create benchmarks that populate a map with $10^3, 10^4, \text{and } 10^5$ bids to measure the exact nanosecond cost of rebalancing.

### 2. HTTP Load Testing (The "System" Level)
Once the algorithm is fast, we must test the **lock contention** in Axum handlers under high load. Use a tool like **Drill** or **Goku**, which are written in Rust for high-throughput benchmarking.
*   **The Goal:** Measure how many Requests Per Second (RPS) `buy` handler can process before the fine-grained locks cause latency spikes (P99s).
*   **Metric to Watch:** **Tail Latency.** In financial systems, the average latency is often a "lie"; we care about the P99 or P99.9—the worst-case delay experienced by users.

### 3. Continuous Profiling (The "Visibility" Level)
In production, we cannot always reproduce performance issues locally. Use **Flamegraphs** (via `cargo-flamegraph`) to visualize exactly where CPU time is being spent—whether it's inside the `BTreeMap` search or waiting for a `Mutex`.
*   **The Goal:** Identify "hot paths" and locking bottlenecks visually.
*   **Tooling:** Use the **Tracing** crate to instrument code. This allows we to collect timing data across `buy` and `buy_impl` boundary without stopping the service.

---

### Production Performance Pyramid
| Test Type | Tool | Focus | When to Run |
| :--- | :--- | :--- | :--- |
| **Micro-bench** | **Criterion** | `buy_impl` algorithmic efficiency | Every PR |
| **Load Test** | **Drill / wrk** | Lock contention & RPS | Release candidates |
| **Profiling** | **Flamegraph** | CPU "Hot paths" | During optimization |


---
### Next:
A sample **Criterion** benchmark script to compare current `retain` logic against a manual removal loop?