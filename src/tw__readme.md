tw_assignment.md
- main full spec doc.

tw_assignment_specs.md  
- minimal summary of business logic doc.

tw_design.md
- self explanatory

tw_known_issues.md
- things omitted, possible improvements.

tw_main.rs
- main app implementation
- important commits:
- tw: Using lock/lockfree per AppState field instead of one Mutex for all AppState
- refactor: replace bids Vec with BTreeMap
- refactor: Full separation between HTTP layer and business logic (_impl fns and thus tests do not see mutex or Arc)

tw_load_test.ps1
- 1000 buy requests, 1 sell and checks

tw_ai_pre_commit_hook.txt
- to check code changes do not violate lock order to prevent deadlock.  

tw_openapi.yaml
- self explanatory
  
tw_perf_testing.md
- performance tests for production
