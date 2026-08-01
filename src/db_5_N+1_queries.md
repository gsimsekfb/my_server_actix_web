# 1+N Queries

## A. What is the problem?

**Setup: Create the tables**
```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL);
```

```sql
INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Carol');

INSERT INTO orders VALUES
  (101, 1, 59.98),
  (102, 2, 15.00),
  (103, 1, 22.50),
  (104, 3, 8.75),
  (105, 2, 42.00);
```

**"Query 1" - Fetch all orders**
```sql
SELECT * FROM orders;
```
Result:
```
id  | user_id | total
101 | 1       | 59.98
102 | 2       | 15.00
103 | 1       | 22.50
104 | 3       | 8.75
105 | 2       | 42.00
```

**"Query N" — for EACH order above, run a separate query to get the user's name**

Usually your app code (or you) is doing this under the hood or accidentally, looping over the 5 orders.  
Now, to imitate this situation, we run 5 more queries, one per row:

```sql
SELECT * FROM users WHERE id = 1;  -- for order 101
SELECT * FROM users WHERE id = 2;  -- for order 102
SELECT * FROM users WHERE id = 1;  -- for order 103
SELECT * FROM users WHERE id = 3;  -- for order 104
SELECT * FROM users WHERE id = 2;  -- for order 105
```

Run them one by one and notice: 1 query for orders + 5 queries for users = **6 total queries** just to display 5 orders with their owner's name. This is the "N+1" — N=5 orders, +1 for the initial fetch.

**The fix — replace all 6 with a single `JOIN`**
```sql
SELECT orders.id AS order_id, orders.total, users.name
FROM orders
JOIN users ON orders.user_id = users.id;
```
Result:
```
order_id | total | name
101      | 59.98 | Alice
102      | 15.00 | Bob
103      | 22.50 | Alice
104      | 8.75  | Carol
105      | 42.00 | Bob
```

Same data, but **1 query instead of 6**. Now imagine 10,000 orders instead of 5 — Step 4's approach becomes 10,001 queries; the `JOIN` stays at 1.


## B. Real production case — ORM lazy loading

Almost nobody writes it manually like this on purpose — it happens **accidentally**, mainly through ORMs (Object-Relational Mappers), where the query-per-row is hidden inside seemingly innocent code.


```python
# Django/Rails-style ORM code, looks totally innocent:
orders = Order.objects.all()  # 1 query: SELECT * FROM orders

for order in orders:
    print(order.user.name)    # each .user access triggers its OWN query!
                              # SELECT * FROM users WHERE id = ?
```


**Why it slips into production:**
1. **Works fine in dev/testing with small data** — 5 orders = 6 queries, nobody notices, feels instant.
2. **Only becomes visible at scale** — 10,000 orders in production = 10,001 queries, suddenly the page takes 8 seconds to load, and the team investigates why.
3. **Nested/serialized API responses** — e.g. an API endpoint returning orders with the user's name embedded (`{"order_id": 101, "user_name": "Alice"}`) — the serializer loops over each order and calls `.user.name`, triggering N+1 without anyone writing an explicit loop of SQL.
4. **Code review blind spot** — the loop looks like plain Python/Ruby object access (`order.user.name`), not obviously "a SQL query," so reviewers don't catch it.

**The fix in ORM terms** — "eager loading," telling the ORM upfront to fetch the join instead of lazy-per-row:
```python
orders = Order.objects.select_related('user').all()  # Django: 1 query, JOIN included
```

So the real-world trigger isn't "someone typed 5 SELECTs" — it's "someone accessed a related field inside a loop without telling the ORM to preload it."

