
## API Versioning

| Approach | Production Use | Complexity | Pros | Cons | When to Use |
|----------|---------------|------------|------|------|-------------|
| URL path `/v1/buy` | Most common | Low | Explicit, easy to route, cacheable | URL pollution | Public APIs |
| Header `API-Version: 1` | Common in enterprise | Medium | Clean URLs | Less visible, harder to test with curl | Enterprise/internal APIs |
| Query param `?version=1` | Least common | Low | Easy to test | Considered unclean, cache issues | Quick prototypes |
| Accept header | Medium | Medium | Strict REST compliant | Verbose, harder to test | REST purists |
| Feature flag | Very common | High | Gradual rollout, no URL changes | Not true versioning, added infrastructure | Gradual rollouts within same version |


```bash

# Using feature flag using JWT token
curl -s -X POST http://localhost:8080/buy -H "Authorization: Bearer eyJh..."   
    // JWT token  
    // header: { "alg": "HS256", "typ": "JWT" }
    // payload: { "sub": "1234567890", "features":["buy_v2"]}  

# Using custom header
curl -H "API-Version: 1" http://localhost:8080/buy

# Using accept header
curl -H "Accept: application/vnd.twn.v2+json" http://localhost:8080/buy
```


## Common issues with API versioning in production:

**-- Clients:**  
Clients never upgrade, so you end up maintaining v1 forever alongside v2 and v3, which becomes a maintenance burden.

Solution:  
Set explicit deprecation timelines and sunset old versions with advance notice.

**-- Development:**  
Breaking changes in shared data structures or business logic must be duplicated across all active versions, making refactoring expensive and error-prone.

### Workaround: Pre API versioning with feature flags 
- Gates new behavior behind a flag rather than a new version, so all clients use the same codebase.
- No `/v2/`, not true versioning. It is for gradual rollouts within same version.


e.g. 

**JWT token example:**  

```
Token:  
header: { "alg": "HS256", "typ": "JWT" }
payload: { "sub": "1234567890", "features":["new_allocation"]}  
secret: "secret"

==================  

curl -s -X POST http://localhost:8080/buy -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZmVhdHVyZXMiOlsibmV3X2FsbG9jYXRpb24iXX0.4Fb63XPR1uY_yUjULL0IifKP5SmkIb1qYvLMyTxRMfk" -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"  

// server:

[src\tw_main.rs:356:13] decode_jwt(token.unwrap()) = Ok(
    Claims {
        sub: "1234567890",
        features: [
            "new_allocation",
        ],
    },
)
[src\tw_main.rs:363:5] has_feature_new_allocation = true

``` 
Token generaion: https://www.jwt.io   

To see impl:  
  - [tw_main.rs](src/tw_main.rs) > `fn buy` service handler
  - [tw_auth.rs](src/tw_auth.rs)  

