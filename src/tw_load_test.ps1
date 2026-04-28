
### 1. BUYs
Add-Type -AssemblyName System.Net.Http
$client = [System.Net.Http.HttpClient]::new()
$tasks = 1..1000 | ForEach-Object {
    $content = [System.Net.Http.StringContent]::new(
        "{`"user`":`"u99`",`"volume`":10,`"price`":3}",
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
# powershell -ExecutionPolicy Bypass -File load_test.ps1
#
# state: AppStateImpl {
#     request_no: 1000,
#     allocations: {
#         "u99": 10000,
#     },
#     supply: 113000,
#     bids: [],
# }

# // checks
# - 1000 buys × 10 volume = 10000 total demand
# - sell 123000, all bids filled → u99 gets 10000
# - leftover supply = 123000 - 10000 = 113000 ✓
# - conservation holds: 10000 + 113000 = 123000 ✓