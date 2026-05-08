# Twin Coding Exercise — Assignment

This is a small, self-contained exercise to assess Rust fundamentals, concurrency thinking, API discipline, and your engineering taste. The **baseline** is intentionally compact; you can **push it as far as you like** with tests, docs, tooling, and performance work.

You should see the produced artifacts as the 

---

## 0) Timebox & Scope

- **Suggested timebox:** 3–6 hours for a solid baseline. Stop whenever you’re proud of it.
- **Extras are optional**—but we *do* look at how you choose to extend things.

---

## 1) Problem

You are building a tiny HTTP service that tracks **bids** for VM capacity and **allocates** integer VM-hours when supply arrives.

### Domain
- A **bid** has:
  - `username` – unique user ID (string)
  - `volume` – requested VM-hours (integer ≥ 0)
  - `price` – max price **per hour** (integer ≥ 0)
- A **supply drop** adds a number of VM-hours to the system.

### Allocation rules
1. **Highest price wins.**
2. **FIFO inside a price level** (earlier bids at the same price fill first).
3. **Partial fills** allowed; unfilled remainder stays open.
4. **Unused supply** persists and **must auto-match** any *subsequent* bids arriving later.

> Note: Rule 4 means a `/buy` arriving when **leftover supply** exists should be allocated immediately (no need to wait for the next `/sell`).

### Example
Events:
- t1: u1 bids 100 @ 3  
- t2: u2 bids 150 @ 2  
- t3: u3 bids 50  @ 4  
- t4: provider sells 250  

Allocation at t4:
- 50 → u3  
- 100 → u1  
- 100 → u2 (u2 still open for 50)

---

## 2) HTTP Interface

Use the exact endpoints and shapes (all JSON):

- **POST `/buy`**  
  Body: `{"username":"u1","volume":100,"price":3}`  
  Behavior: register bid; **immediately allocate** if leftover supply is available.  
  Response: **200 OK** (body ignored).

- **POST `/sell`**  
  Body: `{"volume":250}`  
  Behavior: add supply and allocate to outstanding bids.  
  Response: **200 OK** (body ignored).

- **GET `/allocation?username=u1`**  
  Behavior: return the **integer** total VM-hours allocated to `u1` so far.  
  Responses: **200 OK** with body like `150`, or appropriate **4xx** on error (e.g., missing username).

**Concurrency expectation:** the system will be exercised with many concurrent clients. Your allocation must be **deterministic** and respect the rules above.

---

## 3) Provided Skeleton

You’ll receive a crate named `twin_programming_assignment` with `src/main.rs` containing the three endpoints and **TODO** markers. You may refactor, add modules, and tests, but **do not**:
- change the crate name (`twin_programming_assignment`),
- change the binary name (must build with `cargo build` without renaming),
- change endpoint paths or request/response shapes.

---

## 4) Baseline Acceptance Criteria (what must work)

1. **Correctness under concurrency**  
   - Highest-price-first; FIFO within a price level; partial fills; leftovers roll forward.  
   - New `/buy` must consume any leftover supply immediately.

2. **Determinism**  
   - Tie-breaking within a price level must respect **true arrival order** even under concurrency (e.g., via a monotonic sequence).

3. **API stability**  
   - Endpoints and shapes exactly as specified; status codes correct.

4. **Build & run**  
   - `cargo build` succeeds on stable Rust (≥ 1.78).  
   - `cargo run` starts a server on `0.0.0.0:8080`.

---

## 5) Constraints & Guidance

- **Runtime & libs:** Rust stable, async runtime of your choice (the skeleton uses `actix-web`).  
- **Storage:** **In-memory** only (no external DB, cache, or MQ).  
- **Thread-safety:** Use appropriate synchronization; avoid data races and starvation.  
- **Big-O is fine, clarity is king.** Prefer simple, correct code over premature micro-optimizations.  
- **Input validation:** be reasonable; don’t over-engineer.

---

## 6) What We’d Love To See (push as far as you fancy)

You don’t have to do all of these. Choose the ones that showcase your strengths.

### A) Tests (any mix)
- **Unit tests** for core allocation logic.  
- **Property-based tests** (e.g., proptest) for invariants (monotone allocations, conservation of volume, etc.).  
- **Concurrency tests** (spawn tasks that hammer `/buy` & `/sell`; assert invariants).  
- **Black-box integration tests** using the HTTP layer.

### B) Engineering Hygiene
- A crisp **README**: how to run, test, design notes, assumptions, trade-offs, and known limitations.  
- A small **Design Rationale** section (200–400 words): data structures, concurrency strategy, tie-break approach, failure modes.  
- A short **Decision Log** (`decisions.md` or README section): 3–6 entries capturing key choices and alternatives you considered.
- **Comments in your code** wherever you feel they can help your future colleagues understand your intent better. 

### C) Performance/Observability (lightweight)
- A tiny **load script** (shell, Rust, or k6) with the command you actually ran and a paste of the results.  
- Basic **metrics/logging** you found useful during development.

### D) API Extras (optional)
- A `/stats` debug endpoint (e.g., open orders by price level, leftover supply).  
- Simple **OpenAPI** or a few `curl` examples in the README.

---

## 7) Anti-Boilerplate Signals (please read)

This is an individual exercise. You may consult public docs, but the code and tests must be your own.

To help us understand *your* thinking (and to discourage generic boilerplate), please include:

1. **Design Rationale** (see §6B).  
2. **Decision Log** with timestamps you actually wrote while working.  
3. **At least one test you’re proud of** with a two-sentence comment explaining *what bug it would have caught*.

> These artifacts matter more than polish: we’re looking for judgment, not ceremony.

---

## 8) Submission

- Archive a **single directory named `twin_programming_assignment/`** (the crate root) into `twin_programming_assignment.zip` or `twin_programming_assignment.tar.gz`.  
- The project should build and run with:
  ```bash
  cargo build
  cargo run
  ```
- Include any extra files (README, tests, scripts) inside `twin_programming_assignment/`.

---

## 9) Evaluation Rubric

We score holistically across:

1. **Correctness & determinism (40%)**  
   - Rules respected under concurrent load; no racey tie-breaks; leftovers handled on future buys.

2. **Code clarity & structure (20%)**  
   - Readable, cohesive modules; data structures fit the problem; error handling sensible.

3. **Tests & validation depth (20%)**  
   - Coverage of tricky edges; property or concurrency tests; meaningful assertions.

4. **Engineering taste (10%)**  
   - README, rationale, decision log; small helpful scripts; thoughtful trade-offs.

5. **Pragmatic performance & ops (10%)**  
   - Simple measurements; basic observability; no pathological bottlenecks for this scale.

---

## 10) Quick Local Checklist

- [ ] `buy` honors leftover supply immediately.  
- [ ] Strict price-descending, FIFO within price level.  
- [ ] Sequence/timestamping makes tie-breaks deterministic.  
- [ ] `/allocation` returns a bare integer body.  
- [ ] Concurrent hammer test (even a simple one) behaves as expected.  
- [ ] README explains how to run and what you tested.

---

## 11) Starter cURL (optional)

```bash
# start server in another shell: cargo run

curl -s -X POST localhost:8080/buy  -d '{"username":"u1","volume":100,"price":3}' -H 'content-type: application/json'
curl -s -X POST localhost:8080/buy  -d '{"username":"u2","volume":150,"price":2}' -H 'content-type: application/json'
curl -s -X POST localhost:8080/buy  -d '{"username":"u3","volume":50,"price":4}'  -H 'content-type: application/json'
curl -s -X POST localhost:8080/sell -d '{"volume":250}' -H 'content-type: application/json'
curl -s localhost:8080/allocation?username=u1
```

---

### Final note
Keep it simple, correct, and thoughtfully engineered. Show us how *you* build small, sharp systems.

