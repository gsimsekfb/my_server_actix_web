![Clippy](https://github.com/gsimsekfb/my_server_actix_web/actions/workflows/clippy.yml/badge.svg)


## How to use this library

Each main shows a fundamental architecture or some basic concepts.

How to run each main file:
```
cargo r --bin main_1
cargo r --bin main_4_3
...
```
see more "bins" in Cargo.toml

## Topics

From: https://actix.rs/docs/ 

**Must-knows (build anything with these):**

```
Basics
├── Application      ← App, state, config
├── Extractors       ← how data gets INTO handlers (Path, Query, Json, Form)
└── Handlers         ← how responses go OUT

Advanced
├── Errors           ← Result, AppError, ResponseError
├── Requests         ← reading body, headers, multipart
├── Responses        ← HttpResponse, JSON, streaming
└── Middleware       ← auth, logging, sessions           (mostly for real apps)

Patterns
└── Databases        ← sqlx/postgres, connection pooling         (for real apps)
```

**Nice to know (situational):**

```
Advanced
├── URL Dispatch     ← routing edge cases
├── Testing          ← important but learnable when needed
├── CORS             ← needed for any frontend integration
└── Static Files     ← only if serving files

Protocols
├── WebSockets       ← only if real-time features needed
└── HTTP/2           ← mostly automatic, rarely touched
```

**Learning order:**

```
1. Handlers + Extractors   ← core loop
2. Errors                  ← write safe code early
3. Requests + Responses    ← data in/out
4. Middleware              ← cross-cutting concerns
5. Databases               ← real apps need persistence
6. CORS                    ← when adding a frontend
```

**Bottom line:** Handlers, Extractors, Errors, and Databases get you 80% of production work. Everything else is situational.


