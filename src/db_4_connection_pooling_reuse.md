# Connection Pooling

Connection pooling is reusing a small set of already-open database connections across many requests, instead of opening a brand-new connection for every single request.

**Why it matters:** opening a DB connection is expensive (TCP handshake, auth, session setup) — doing that on every request would kill performance under load.

**How it works:**
```
[App] --request--> [Connection Pool: 10 open connections]
                         |
                    borrow one, use it, return it
```

Example flow:
1. App starts, creates a pool of e.g. 10 persistent connections to Postgres
2. Request A comes in → borrows connection #3 from the pool → runs its query → returns connection #3 to the pool
3. Request B comes in → borrows connection #3 (or #7, whichever is free) → runs its query → returns it

If all 10 connections are busy and request C comes in, it waits in a queue until one frees up (or the pool grows, up to a configured max).

**Important:**  
A pool's connections aren't infinitely reused: they're typically recycled after a max lifetime or idle timeout to avoid issues like stale connections or DB-side resource leaks.
