## Simplest First Load Test: Throughput & Latency Under Concurrency Sweep

**One test, one question:** *"How does my server's latency change as I increase concurrent connections?"*

Run the **same endpoint** at increasing concurrency levels (e.g., 1 → 10 → 50 → 100 → 500) and record **requests/sec** and **p99 latency** at each level. That's it.

**Why this one first:**
- It immediately reveals your **saturation point** — where latency starts bending upward
- It catches the biggest problems: lock contention, thread pool exhaustion, backpressure failures
- It requires zero scripting — a single CLI command per level

**Example with `hey`** (simplest tool to install and use):

```bash
# 10 concurrent connections, 10 seconds, unlimited requests.
hey -c 10 -z 10s http://localhost:8080/your_endpoint

# 100 concurrent connections, 10 seconds, unlimited requests.
hey -c 100 -z 10s http://localhost:8080/your_endpoint

# 500 concurrent connections, 10 seconds, unlimited requests.
hey -c 500 -z 10s http://localhost:8080/your_endpoint
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