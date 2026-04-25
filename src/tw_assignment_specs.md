
# SPECS

## 1) Problem

### Allocation rules
- Highest price wins.
- FIFO inside a price level (earlier bids at the same price fill first).
- Partial fills allowed; unfilled remainder stays open.
- Unused supply persists and must auto-match any subsequent bids arriving later.

Note: Rule 4 means a /buy arriving when leftover supply exists should be allocated immediately (no need to wait for the next /sell).

    Example
    Events:

    t1: u1 bids 100 @ 3
    t2: u2 bids 150 @ 2
    t3: u3 bids 50 @ 4
    t4: provider sells 250
    Allocation at t4:

    50 → u3
    100 → u1
    100 → u2 (u2 still open for 50)


## 4) Baseline Acceptance Criteria (what must work)

1. Correctness under concurrency

Highest-price-first; FIFO within a price level; partial fills; leftovers roll forward.
New /buy must consume any leftover supply immediately.

2. Determinism

Tie-breaking within a price level must respect true arrival order even under concurrency (e.g., via a monotonic sequence).

3. API stability

Endpoints and shapes exactly as specified; status codes correct.

4. Build & run

cargo build succeeds on stable Rust (≥ 1.78).
cargo run starts a server on 0.0.0.0:8080.


## 10) Quick Local Checklist
- buy honors leftover supply immediately.
- Strict price-descending, FIFO within price level.
- Sequence/timestamping makes tie-breaks deterministic.
- /allocation returns a bare integer body.
- Concurrent hammer test (even a simple one) behaves as expected.
- README explains how to run and what you tested.
