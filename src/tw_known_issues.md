## Known Issues

### Critical

- **No persistence** — full state lost on crash/restart. WAL or event sourcing required for production.
- **`unwrap()` on lock** — poison causes panic, crashing the worker thread. Needs explicit poison handling.
- **`retain` no early exit** — iterates all bids after supply hits zero. Under high bid volume this wastes CPU on every `/sell`.

---

### High

- **Input validation missing** — no checks for `volume=0`, `price=0`, or unreasonably large values. Should return 4xx at API boundary.
- **No overflow protection** — `supply += volume` can overflow with no guard. `checked_add` should be used.
- **`buy_seq_no` read after increment** — `fetch_add` returns old value; code calls `load` separately after. Race window between increment and read — seq assigned to bid could be wrong under concurrency.
- **`username` field mismatch** — spec says `username` but struct uses `user`. Will break the spec's curl examples.

---

### Medium

- **No request timeout** — long-held locks could starve other requests indefinitely.
- **`dbg!` left in production code** — `sell_impl` has `dbg!(total_alloc)` which prints to stderr in production.
- **`println!` in buy_impl** — same issue, should use `log::debug!` behind a feature flag.
- **`my_middleware` is a no-op** — registered but does nothing, confusing to future readers.
- **`index` endpoint exposes full state** — debug endpoint leaks internal state in production.

---

### Low

- **`#[allow(dead_code)]` at crate level** — masks unused code warnings, should be removed before production.
- **`ordered_locks_buy` and `ordered_locks_sell` are identical** — could be a single `ordered_locks` fn.
- **No OpenAPI spec** — makes onboarding harder for API consumers.
- **No graceful shutdown** — `handle` is created but never used for graceful drain.
- **`tests_lib` not gated with `#[cfg(test)]`** — test helpers compiled into production binary.