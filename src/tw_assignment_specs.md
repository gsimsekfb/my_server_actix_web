
# SPECS

## Allocation rules
- Highest price wins.
- FIFO inside a price level (earlier bids at the same price fill first).
- Partial fills allowed; unfilled remainder stays open.
- Unused supply persists and must auto-match any subsequent bids arriving later.

Note: Rule 4 means a /buy arriving when leftover supply exists should be allocated immediately (no need to wait for the next /sell).

