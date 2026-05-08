# See section "3. example run" first 

### 1. BUYs
Add-Type -AssemblyName System.Net.Http
$client = [System.Net.Http.HttpClient]::new()
$tasks = 1..1000 | ForEach-Object {
    $content = [System.Net.Http.StringContent]::new(
        "{`"user`":`"u1`",`"volume`":10,`"price`":3}",
        [System.Text.Encoding]::UTF8,
        "application/json"
    )
    $client.PostAsync("http://localhost:8080/buy", $content)
}
$tasks | ForEach-Object { $_.Wait() }


### 2. SELL
# curl -s -X POST localhost:8080/sell -d '{"volume":123000}' -H 'content-type: application/json'
# at the end of script
Invoke-RestMethod -Method Post -Uri "http://localhost:8080/sell" `
    -ContentType "application/json" `
    -Body '{"volume":123000}'


### 3. example run:
#
# // what's happening & checks:
# - u1: 1000 buy requests each (10 volume) = 10_000 total demand
# - sell 123_000 volume, triggers all bids auto filled → u1 gets 10_000
# - leftover supply = 123_000 - 10_000 = 113_000 ✓
# - supply conservation holds: 10_000 + 113_000 = 123_000 ✓
# 
# powershell -ExecutionPolicy Bypass -File load_test.ps1
#
# state: AppStateImpl {
#     request_no: 1000,
#     allocations: {
#         "u1": 10000,
#     },
#     supply: 113000,
#     bids: [],
# }
