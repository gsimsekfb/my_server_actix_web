![Clippy](https://github.com/gsimsekfb/my_server_actix_web/actions/workflows/clippy.yml/badge.svg)

## my_server_actix_web
This is an educational library with working self-contained examples with lots of comments.

**Main topic:**  
backend fundamentals using actix_web 

> [main_2_app_state_simple.rs](src/main_2_app_state_simple.rs)  
> [main_3_app_state_mutable_global.rs](src/main_3_app_state_mutable_global.rs)  
> [main_4_1_extractors_path.rs](src/main_4_1_extractors_path.rs)  
> [main_4_2_extractors_query_json_form.rs](src/main_4_2_extractors_query_json_form.rs)   
> [main_4_3_extractors_app_state.rs](src/main_4_3_extractors_app_state.rs)  
> [main_5_HttpResponse_Responder_aka_HttpReq_handlers.rs](src/main_5_HttpResponse_Responder_aka_HttpReq_handlers.rs)  
> [main_6_1_middleware_basics.rs](src/main_6_1_middleware_basics.rs)  
> [main_6_2_default_headers_middleware.rs](src/main_6_2_default_headers_middleware.rs)  
> [main_6_3_Error_Handlers_middleware.rs](src/main_6_3_Error_Handlers_middleware.rs)  
> [main_6_4_logging_middleware.rs](src/main_6_4_logging_middleware.rs)  
> [main_6_5_user_sessions_middleware.rs](src/main_6_5_user_sessions_middleware.rs)  
> [main_7_0_Errors.rs](src/main_7_0_Errors.rs)  
> [main_7_1_Error_Logging.rs](src/main_7_1_Error_Logging.rs)  
> [main_8_0_Testing.rs](src/main_8_0_Testing.rs)  
> [main_8_1_Testing_Streams.rs](src/main_8_1_Testing_Streams.rs)  
> [tw_ai_pre_commit_hook.txt](src/tw_ai_pre_commit_hook.txt)   
> [tw_assignment_specs.md](src/tw_assignment_specs.md)  
> [tw_buys.bat](src/tw_buys.bat)  
> [tw_design.md](src/tw_design.md)  
> [tw_known_issues.md](src/tw_known_issues.md)  
> [tw_load_test.ps1](src/tw_load_test.ps1)  
> [tw_main.rs](src/tw_main.rs)  
> [tw_openapi.yaml](src/tw_openapi.yaml)  
> [tw_perf_testing.md](src/tw_perf_testing.md)  
> [tw__readme.md](src/tw__readme.md)    


## How to use this library

Each main shows a fundamental architecture or some basic concepts.

How to run each main file:
```
cargo r --bin main_2
cargo r --bin main_4_1
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


