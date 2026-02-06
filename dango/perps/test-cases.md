# Test Cases for Perps Contract

## SubmitOrder

### Test Parameters

```plain
oracle_price = 100
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
max_abs_oi = 500
```

### Key Formulas

- `marginal_premium = clamp(skew/K, -M, M)`
- `marginal_price = oracle_price * (1 + marginal_premium)`
- For buy: `target_price = marginal_price * (1 + max_slippage)`
- For sell: `target_price = marginal_price * (1 - max_slippage)`
- `exec_price = oracle_price * (1 + clamp((skew + fill/2)/K, -M, M))`
- **Price check (all-or-nothing):** For buy: `exec_price <= target_price`; For sell: `exec_price >= target_price`

### Test Case Summary Table

| #   | Scenario                          | user_pos | long_oi | short_oi | skew | size | reduce_only | kind         | fill     | exec_price | Limiting        |
| --- | --------------------------------- | -------- | ------- | -------- | ---- | ---- | ----------- | ------------ | -------- | ---------- | --------------- |
| 1   | New long, unconstrained           | 0        | 100     | -100     | 0    | +50  | false       | Market(5%)   | **+50**  | 102.5      | None            |
| 2   | New short, unconstrained          | 0        | 100     | -100     | 0    | -50  | false       | Market(5%)   | **-50**  | 97.5       | None            |
| 3   | New long, OI blocked              | 0        | 480     | -100     | 380  | +50  | false       | Market(5%)   | **0**    | -          | OI (revert)     |
| 4   | New short, OI blocked             | 0        | 100     | -480     | -380 | -50  | false       | Market(5%)   | **0**    | -          | OI (revert)     |
| 5   | Close long fully                  | +100     | 200     | -100     | 100  | -100 | false       | Market(1%)   | **-100** | 105        | None (closing)  |
| 6   | Close short fully                 | -100     | 100     | -200     | -100 | +100 | false       | Market(1%)   | **+100** | 95         | None (closing)  |
| 7   | Flip, reduce_only=false, OK       | +100     | 200     | -100     | 100  | -150 | false       | Market(5%)   | **-150** | 102.5      | None            |
| 8   | Flip, reduce_only=false, OI block | +100     | 200     | -480     | -280 | -150 | false       | Market(5%)   | **0**    | -          | OI (revert all) |
| 9   | Flip, reduce_only=true            | +100     | 200     | -480     | -280 | -150 | true        | Market(5%)   | **-100** | 95         | reduce_only     |
| 10  | Slippage exceeded                 | 0        | 100     | -100     | 0    | +100 | false       | Market(1%)   | **0**    | -          | Price           |
| 11  | Limit buy, price exceeded         | 0        | 100     | -100     | 0    | +50  | false       | Limit(101.5) | **0**    | -          | Limit price     |
| 12  | Limit buy, below marginal         | 0        | 100     | -100     | 0    | +50  | false       | Limit(99)    | **0**    | -          | Limit<marginal  |
| 13  | Close at max OI                   | +100     | 500     | -100     | 400  | -100 | false       | Market(5%)   | **-100** | 105        | None (closing)  |

### Detailed Calculations

#### Test 1: New long, unconstrained

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, reduce_only=false, slippage=5%

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** room = 500-100 = 400, result = min(50, 400) = **+50**
3. **OI violated?** max_opening (50) < opening (50)? No → OI OK
4. **fill_size:** +50
5. **compute_target_price:**
   - marginal_premium = clamp(0/1000, -0.05, 0.05) = 0
   - marginal_price = 100 * 1.0 = 100
   - target_price = 100 * 1.05 = 105
6. **compute_exec_price(+50, skew=0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) <= target_price (105) → **PASS**
8. **fill_size = +50** ✓

#### Test 2: New short, unconstrained

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=-50, reduce_only=false, slippage=5%

1. **decompose_fill(-50, 0):** closing=0, opening=-50
2. **compute_max_opening_from_oi(-50):** room = 500-100 = 400, result = max(-50, -400) = **-50**
3. **OI violated?** max_opening (-50) > opening (-50)? No → OI OK
4. **fill_size:** -50
5. **compute_target_price:**
   - marginal_price = 100
   - target_price = 100 * 0.95 = 95
6. **compute_exec_price(-50, skew=0):**
   - premium = clamp((0 + (-50)/2) / 1000, -0.05, 0.05) = -0.025
   - exec_price = 100 * 0.975 = **97.5**
7. **Price check:** exec_price (97.5) >= target_price (95) → **PASS**
8. **fill_size = -50** ✓

#### Test 3: New long, OI blocked

**Input:** user_pos=0, long_oi=480, short_oi=-100, skew=380, size=+50, reduce_only=false, slippage=5%

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** room = 500-480 = 20, result = min(50, 20) = **+20**
3. **OI violated?** max_opening (20) < opening (50)? Yes → OI violated
4. **reduce_only=false:** entire order reverts
5. **fill_size = 0** ✓

#### Test 4: New short, OI blocked

**Input:** user_pos=0, long_oi=100, short_oi=-480, skew=-380, size=-50, reduce_only=false, slippage=5%

1. **decompose_fill(-50, 0):** closing=0, opening=-50
2. **compute_max_opening_from_oi(-50):** room = 500-480 = 20, result = max(-50, -20) = **-20**
3. **OI violated?** max_opening (-20) > opening (-50)? Yes → OI violated
4. **reduce_only=false:** entire order reverts
5. **fill_size = 0** ✓

#### Test 5: Close long fully (closing always allowed)

**Input:** user_pos=+100, long_oi=200, short_oi=-100, skew=100, size=-100, reduce_only=false, slippage=1%

1. **decompose_fill(-100, +100):**
   - size < 0, user_pos > 0 → closing = max(-100, -100) = -100, opening = 0
2. **compute_max_opening_from_oi(0):** returns **0**
3. **OI violated?** No opening, so no violation
4. **fill_size:** -100
5. **compute_target_price:**
   - marginal_premium = clamp(100/1000, -0.05, 0.05) = 0.05 (clamped)
   - marginal_price = 100 * 1.05 = 105
   - target_price = 105 * 0.99 = 103.95
6. **compute_exec_price(-100, skew=100):**
   - premium = clamp((100 + (-100)/2) / 1000, -0.05, 0.05) = clamp(0.05, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) >= target_price (103.95) → **PASS**
8. **fill_size = -100** ✓

#### Test 6: Close short fully (closing always allowed)

**Input:** user_pos=-100, long_oi=100, short_oi=-200, skew=-100, size=+100, reduce_only=false, slippage=1%

1. **decompose_fill(+100, -100):**
   - size > 0, user_pos < 0 → closing = min(100, 100) = +100, opening = 0
2. **compute_max_opening_from_oi(0):** returns **0**
3. **OI violated?** No opening, so no violation
4. **fill_size:** +100
5. **compute_target_price:**
   - marginal_premium = clamp(-100/1000, -0.05, 0.05) = -0.05 (clamped)
   - marginal_price = 100 * 0.95 = 95
   - target_price = 95 * 1.01 = 95.95
6. **compute_exec_price(+100, skew=-100):**
   - premium = clamp((-100 + 100/2) / 1000, -0.05, 0.05) = clamp(-0.05, -0.05, 0.05) = -0.05
   - exec_price = 100 * 0.95 = **95**
7. **Price check:** exec_price (95) <= target_price (95.95) → **PASS**
8. **fill_size = +100** ✓

#### Test 7: Flip long→short, reduce_only=false, OI OK

**Input:** user_pos=+100, long_oi=200, short_oi=-100, skew=100, size=-150, reduce_only=false, slippage=5%

1. **decompose_fill(-150, +100):**
   - closing = max(-150, -100) = -100, opening = -150 - (-100) = -50
2. **compute_max_opening_from_oi(-50):**
   - short_oi.abs() = 100, room = 500 - 100 = 400
   - result = max(-50, -400) = **-50**
3. **OI violated?** max_opening (-50) > opening (-50)? No → OI OK
4. **fill_size:** -150
5. **compute_target_price:**
   - marginal_premium = clamp(100/1000, -0.05, 0.05) = 0.05 (clamped)
   - marginal_price = 100 * 1.05 = 105
   - target_price = 105 * 0.95 = 99.75
6. **compute_exec_price(-150, skew=100):**
   - premium = clamp((100 + (-150)/2) / 1000, -0.05, 0.05) = clamp(0.025, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) >= target_price (99.75) → **PASS**
8. **fill_size = -150** ✓

#### Test 8: Flip, reduce_only=false, OI blocked

**Input:** user_pos=+100, long_oi=200, short_oi=-480, skew=-280, size=-150, reduce_only=false, slippage=5%

1. **decompose_fill(-150, +100):**
   - closing = max(-150, -100) = -100, opening = -150 - (-100) = -50
2. **compute_max_opening_from_oi(-50):**
   - short_oi.abs() = 480, room = 500 - 480 = 20
   - result = max(-50, -20) = **-20** (limited!)
3. **OI violated?** max_opening (-20) > opening (-50)? Yes → OI violated
4. **reduce_only=false:** entire order (including closing) reverts
5. **fill_size = 0** ✓

#### Test 9: Flip, reduce_only=true

**Input:** user_pos=+100, long_oi=200, short_oi=-480, skew=-280, size=-150, reduce_only=true, slippage=5%

1. **decompose_fill(-150, +100):**
   - closing = max(-150, -100) = -100, opening = -150 - (-100) = -50
2. **compute_max_opening_from_oi(-50):**
   - short_oi.abs() = 480, room = 500 - 480 = 20
   - result = max(-50, -20) = **-20** (limited!)
3. **OI violated?** max_opening (-20) > opening (-50)? Yes → OI violated
4. **reduce_only=true:** execute only closing portion, discard opening
5. **fill_size:** -100 (closing only)
6. **compute_target_price:**
   - marginal_premium = clamp(-280/1000, -0.05, 0.05) = -0.05 (clamped)
   - marginal_price = 100 * 0.95 = 95
   - target_price = 95 * 0.95 = 90.25
7. **compute_exec_price(-100, skew=-280):**
   - premium = clamp((-280 + (-100)/2) / 1000, -0.05, 0.05) = clamp(-0.33, -0.05, 0.05) = -0.05
   - exec_price = 100 * 0.95 = **95**
8. **Price check:** exec_price (95) >= target_price (90.25) → **PASS**
9. **fill_size = -100** ✓

#### Test 10: Buy, slippage exceeded (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+100, reduce_only=false, slippage=1%

1. **decompose_fill(+100, 0):** closing=0, opening=+100
2. **compute_max_opening_from_oi(+100):** room = 400, result = **+100**
3. **OI violated?** No
4. **fill_size:** +100
5. **compute_target_price:**
   - marginal_premium = 0
   - marginal_price = 100
   - target_price = 100 * 1.01 = 101
6. **compute_exec_price(+100, 0):**
   - premium = clamp((0 + 100/2) / 1000, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) > target_price (101) → **FAIL**
8. **fill_size = 0** ✓ (order rejected due to slippage)

#### Test 11: Limit buy, price exceeded (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, reduce_only=false, limit_price=101.5

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** result = **+50**
3. **OI violated?** No
4. **fill_size:** +50
5. **compute_target_price:** target_price = 101.5
6. **compute_exec_price(+50, 0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) > target_price (101.5) → **FAIL**
8. **fill_size = 0** ✓ (order stored as GTC for later fulfillment)

#### Test 12: Limit buy, below marginal (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, reduce_only=false, limit_price=99

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** result = **+50**
3. **OI violated?** No
4. **fill_size:** +50
5. **compute_target_price:** target_price = 99
6. **compute_exec_price(+50, 0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) > target_price (99) → **FAIL**
8. **fill_size = 0** ✓ (order stored as GTC for later fulfillment)

#### Test 13: Close at max OI (closing always allowed)

**Input:** user_pos=+100, long_oi=500, short_oi=-100, skew=400, size=-100, reduce_only=false, slippage=5%

1. **decompose_fill(-100, +100):**
   - closing = max(-100, -100) = -100, opening = 0
2. **compute_max_opening_from_oi(0):** returns **0**
3. **OI violated?** No opening, so no violation
4. **fill_size:** -100
5. **compute_target_price:**
   - marginal_premium = clamp(400/1000, -0.05, 0.05) = 0.05 (clamped)
   - marginal_price = 100 * 1.05 = 105
   - target_price = 105 * 0.95 = 99.75
6. **compute_exec_price(-100, skew=400):**
   - premium = clamp((400 + (-100)/2) / 1000, -0.05, 0.05) = clamp(0.35, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) >= target_price (99.75) → **PASS**
8. **fill_size = -100** ✓

### Analysis Notes

#### Note 1: reduce_only behavior

When `reduce_only = true`:

- The closing portion is always executed
- The opening portion is discarded (not executed)
- This allows users to close positions even when OI constraints would block new positions

When `reduce_only = false`:

- If opening would violate OI, the entire order reverts (all-or-nothing)
- This ensures users get their full intended exposure or nothing

#### Note 2: Closing always allowed

The decomposition into closing/opening portions ensures that closing an existing position is never blocked by OI constraints, even when those constraints are at their limits. This is important because:

1. Closing reduces risk in the system
2. Users should always be able to exit positions

### Conclusion

All test cases verify the algorithm correctly handles:

| Category               | Behavior                        | Tests    |
| ---------------------- | ------------------------------- | -------- |
| Unconstrained orders   | Full fill                       | 1, 2     |
| OI blocked (opening)   | Revert if reduce_only=false     | 3, 4     |
| Closing positions      | Always allowed                  | 5, 6, 13 |
| Flip, OI OK            | Full fill                       | 7        |
| Flip, OI blocked       | Revert all if reduce_only=false | 8        |
| Flip, reduce_only=true | Close only, discard opening     | 9        |
| Slippage exceeded      | Reject (all-or-nothing)         | 10       |
| Limit price exceeded   | Fill = 0, stored as GTC         | 11       |
| Limit below market     | Fill = 0, stored as GTC         | 12       |

## GTC Order Fulfillment

Test cases for the `fulfill_limit_orders_for_pair` algorithm that processes pending GTC limit orders when oracle prices change.

### Test Parameters

```plain
oracle_price = 100 (initial, changes per test)
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
max_abs_oi = 500
```

### Key Formulas

- `marginal_price = oracle_price * (1 + clamp(skew/K, -M, M))`
- `exec_price = oracle_price * (1 + clamp((skew + size/2)/K, -M, M))`
- For buy orders: `limit_price >= marginal_price` (cutoff), `exec_price <= limit_price` (fill check)
- For sell orders: `limit_price <= marginal_price` (cutoff), `exec_price >= limit_price` (fill check)

### Test Case Summary Table

| #   | Scenario                       | Key Behavior                                       |
| --- | ------------------------------ | -------------------------------------------------- |
| 1   | Single buy order filled        | Oracle drops → limit_price >= marginal_price       |
| 2   | Single sell order filled       | Oracle rises → limit_price <= marginal_price       |
| 3   | Buy order unfillable (cutoff)  | limit_price < marginal_price → break immediately   |
| 4   | Sell order unfillable (cutoff) | limit_price > marginal_price → break immediately   |
| 5   | Buy order too large (size)     | exec_price > limit_price → continue scanning       |
| 6   | Price-time priority (buys)     | Higher price fills first; same price → older first |
| 7   | Price-time priority (sells)    | Lower price fills first; same price → older first  |
| 8   | Skew changes affect subsequent | First fill changes skew, second order unfillable   |
| 9   | OI blocks, reduce_only=false   | Order skipped entirely                             |
| 10  | OI blocks, reduce_only=true    | Only closing portion filled, order updated         |
| 11  | Mixed buy and sell orders      | Interleaved by timestamp                           |
| 12  | Interleaved multiple orders    | Timestamp ordering across multiple orders          |
| 13  | Timestamp tiebreaker           | Same timestamp: buy wins                           |
| 14  | Cutoff unblocked by other side | Sell changes skew, making blocked buy fillable     |

### Detailed Calculations

#### Test 1: Single buy order filled

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS: [{limit_price=103, size=+50, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price drops from 100 → 98

**Calculation:**

1. marginal_price = 98 * (1 + clamp(0/1000, -0.05, 0.05)) = 98 * 1.0 = **98**
2. Cutoff check: limit_price(103) >= marginal_price(98) → **PASS** (continue)
3. exec_price = 98 * (1 + clamp((0 + 50/2)/1000, -0.05, 0.05)) = 98 * 1.025 = **100.45**
4. Price check: exec_price(100.45) <= limit_price(103) → **PASS** (fill)
5. decompose_fill(+50, 0): closing=0, opening=+50
6. OI check: room = 500-100 = 400, max_opening=+50, not violated
7. Fill order, update skew: skew → 0+50 = 50

**Result:**

- long_oi=150, short_oi=-100, skew=50
- BUY_ORDERS: [] (order removed)
- exec_price = **100.45**

---

#### Test 2: Single sell order filled

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- SELL_ORDERS: [{limit_price=97, size=-50, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price rises from 100 → 102

**Calculation:**

1. marginal_price = 102 * (1 + clamp(0/1000, -0.05, 0.05)) = 102 * 1.0 = **102**
2. Cutoff check: limit_price(97) <= marginal_price(102) → **PASS** (continue)
3. exec_price = 102 * (1 + clamp((0 + (-50)/2)/1000, -0.05, 0.05)) = 102 * 0.975 = **99.45**
4. Price check: exec_price(99.45) >= limit_price(97) → **PASS** (fill)
5. decompose_fill(-50, 0): closing=0, opening=-50
6. OI check: room = 500-100 = 400, max_opening=-50, not violated
7. Fill order, update skew: skew → 0+(-50) = -50

**Result:**

- long_oi=100, short_oi=-150, skew=-50
- SELL_ORDERS: [] (order removed)
- exec_price = **99.45**

---

#### Test 3: Buy order unfillable (cutoff)

**Initial State:**

- long_oi=200, short_oi=-100, skew=100
- BUY_ORDERS: [{limit_price=104, size=+50, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Calculation:**

1. marginal_price = 100 * (1 + clamp(100/1000, -0.05, 0.05)) = 100 * 1.05 = **105** (clamped)
2. Cutoff check: limit_price(104) >= marginal_price(105) → **FAIL** (break)
3. No further processing

**Result:**

- long_oi=200, short_oi=-100, skew=100 (unchanged)
- BUY_ORDERS: [{limit_price=104, size=+50}] (order remains)
- No fill

---

#### Test 4: Sell order unfillable (cutoff)

**Initial State:**

- long_oi=100, short_oi=-200, skew=-100
- SELL_ORDERS: [{limit_price=96, size=-50, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Calculation:**

1. marginal_price = 100 * (1 + clamp(-100/1000, -0.05, 0.05)) = 100 * 0.95 = **95** (clamped)
2. Cutoff check: limit_price(96) <= marginal_price(95) → **FAIL** (break)
3. No further processing

**Result:**

- long_oi=100, short_oi=-200, skew=-100 (unchanged)
- SELL_ORDERS: [{limit_price=96, size=-50}] (order remains)
- No fill

---

#### Test 5: Buy order too large (continue scanning)

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS (sorted by descending limit_price):
  - Order A: {limit_price=102, size=+100, user_pos=0, reduce_only=false}
  - Order B: {limit_price=101, size=+20, user_pos=0, reduce_only=false}

**Trigger:** Oracle price = 100

**Calculation for Order A:**

1. marginal_price = 100 * 1.0 = **100**
2. Cutoff check: limit_price(102) >= marginal_price(100) → **PASS**
3. exec_price = 100 * (1 + clamp((0 + 100/2)/1000, -0.05, 0.05)) = 100 * 1.05 = **105**
4. Price check: exec_price(105) <= limit_price(102) → **FAIL** (skip, continue)

**Calculation for Order B:**

1. marginal_price = 100 * 1.0 = **100** (skew unchanged, Order A wasn't filled)
2. Cutoff check: limit_price(101) >= marginal_price(100) → **PASS**
3. exec_price = 100 * (1 + clamp((0 + 20/2)/1000, -0.05, 0.05)) = 100 * 1.01 = **101**
4. Price check: exec_price(101) <= limit_price(101) → **PASS** (fill)
5. Fill order, update skew: skew → 0+20 = 20

**Result:**

- long_oi=120, short_oi=-100, skew=20
- BUY_ORDERS: [{limit_price=102, size=+100}] (Order A remains)
- Order B filled at exec_price = **101**

---

#### Test 6: Price-time priority (buys)

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS (unsorted input):
  - Order A: {limit_price=103, created_at=t2, size=+10, user_pos=0}
  - Order B: {limit_price=105, created_at=t1, size=+10, user_pos=0}
  - Order C: {limit_price=103, created_at=t1, size=+10, user_pos=0}

**Expected Processing Order:** B (highest price), then C (same price as A, but older), then A

**Trigger:** Oracle price = 100

**Processing Order B first (limit=105):**

1. marginal_price = 100, limit(105) >= marginal(100) → continue
2. exec_price = 100 * 1.005 = 100.5, exec(100.5) <= limit(105) → fill
3. skew → 10

**Processing Order C next (limit=103, t=t1):**

1. marginal_price = 100 * (1 + 0.01) = 101, limit(103) >= marginal(101) → continue
2. exec_price = 100 * (1 + clamp((10+10/2)/1000, -0.05, 0.05)) = 100 * 1.015 = 101.5
3. exec(101.5) <= limit(103) → fill
4. skew → 20

**Processing Order A last (limit=103, t=t2):**

1. marginal_price = 100 * (1 + 0.02) = 102, limit(103) >= marginal(102) → continue
2. exec_price = 100 * (1 + clamp((20+10/2)/1000, -0.05, 0.05)) = 100 * 1.025 = 102.5
3. exec(102.5) <= limit(103) → fill
4. skew → 30

**Result:**

- All orders filled in order: B → C → A
- Final skew = 30

---

#### Test 7: Price-time priority (sells)

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- SELL_ORDERS (unsorted input):
  - Order A: {limit_price=97, created_at=t2, size=-10, user_pos=0}
  - Order B: {limit_price=95, created_at=t1, size=-10, user_pos=0}
  - Order C: {limit_price=97, created_at=t1, size=-10, user_pos=0}

**Expected Processing Order:** B (lowest price), then C (same price as A, but older), then A

**Trigger:** Oracle price = 100

**Processing Order B first (limit=95):**

1. marginal_price = 100, limit(95) <= marginal(100) → continue
2. exec_price = 100 * 0.995 = 99.5, exec(99.5) >= limit(95) → fill
3. skew → -10

**Processing Order C next (limit=97, t=t1):**

1. marginal_price = 100 * (1 - 0.01) = 99, limit(97) <= marginal(99) → continue
2. exec_price = 100 * (1 + clamp((-10-10/2)/1000, -0.05, 0.05)) = 100 * 0.985 = 98.5
3. exec(98.5) >= limit(97) → fill
4. skew → -20

**Processing Order A last (limit=97, t=t2):**

1. marginal_price = 100 * (1 - 0.02) = 98, limit(97) <= marginal(98) → continue
2. exec_price = 100 * (1 + clamp((-20-10/2)/1000, -0.05, 0.05)) = 100 * 0.975 = 97.5
3. exec(97.5) >= limit(97) → fill
4. skew → -30

**Result:**

- All orders filled in order: B → C → A
- Final skew = -30

---

#### Test 8: Skew changes affect subsequent orders

**Initial State:**

- long_oi=140, short_oi=-100, skew=40
- BUY_ORDERS (sorted by descending limit_price):
  - Order A: {limit_price=105, size=+20, user_pos=0, reduce_only=false}
  - Order B: {limit_price=104.5, size=+20, user_pos=0, reduce_only=false}

**Trigger:** Oracle price = 100

**Processing Order A (limit=105):**

1. marginal_price = 100 * (1 + clamp(40/1000, -0.05, 0.05)) = 100 * 1.04 = **104**
2. Cutoff check: limit(105) >= marginal(104) → **PASS**
3. exec_price = 100 * (1 + clamp((40+20/2)/1000, -0.05, 0.05)) = 100 * 1.05 = **105** (clamped)
4. Price check: exec(105) <= limit(105) → **PASS** (fill)
5. skew → 40+20 = 60

**Processing Order B (limit=104.5):**

1. marginal_price = 100 * (1 + clamp(60/1000, -0.05, 0.05)) = 100 * 1.05 = **105** (clamped)
2. Cutoff check: limit(104.5) >= marginal(105) → **FAIL** (break)

**Result:**

- long_oi=160, short_oi=-100, skew=60
- BUY_ORDERS: [{limit_price=104.5, size=+20}] (Order B remains)
- Order A filled at exec_price = **105**

---

#### Test 9: OI blocks, reduce_only=false

**Initial State:**

- long_oi=480, short_oi=-100, skew=380
- BUY_ORDERS: [{limit_price=110, size=+50, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Calculation:**

1. marginal_price = 100 * 1.05 = **105** (clamped at max premium)
2. Cutoff check: limit(110) >= marginal(105) → **PASS**
3. exec_price = 100 * (1 + clamp((380+50/2)/1000, -0.05, 0.05)) = 100 * 1.05 = **105**
4. Price check: exec(105) <= limit(110) → **PASS** (would fill)
5. decompose_fill(+50, 0): closing=0, opening=+50
6. OI check: room = 500-480 = 20, max_opening=+20 < opening(+50) → **OI VIOLATED**
7. reduce_only=false → skip order entirely (continue scanning)

**Result:**

- long_oi=480, short_oi=-100, skew=380 (unchanged)
- BUY_ORDERS: [{limit_price=110, size=+50}] (order remains)
- No fill

---

#### Test 10: OI blocks, reduce_only=true

**Initial State:**

- long_oi=480, short_oi=-100, skew=380
- BUY_ORDERS: [{limit_price=110, size=+150, user_pos=-100, reduce_only=true}]

**Trigger:** Oracle price = 100

**Calculation:**

1. marginal_price = 100 * 1.05 = **105** (clamped)
2. Cutoff check: limit(110) >= marginal(105) → **PASS**
3. decompose_fill(+150, -100): closing=min(150,100)=+100, opening=+50
4. OI check: room = 500-480 = 20, max_opening=+20 < opening(+50) → **OI VIOLATED**
5. reduce_only=true → execute only closing portion (+100)
6. exec_price for closing = 100 * (1 + clamp((380+100/2)/1000, -0.05, 0.05)) = 100 * 1.05 = **105**
7. Price check: exec(105) <= limit(110) → **PASS**
8. Fill closing portion, update skew: skew → 380+100 = 480

**Result:**

- long_oi=480, short_oi=0, skew=480 (user closed their short, reducing short_oi)
- BUY_ORDERS: [{limit_price=110, size=+50}] (order updated with remaining size)
- Partial fill at exec_price = **105**

---

#### Test 11: Mixed buy and sell orders (interleaved by timestamp)

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS: [{limit_price=103, size=+30, created_at=t2, user_pos=0, reduce_only=false}]
- SELL_ORDERS: [{limit_price=97, size=-30, created_at=t1, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Interleaved Processing (sell is older, processed first):**

**Step 1: Process Sell (t=t1)**

1. marginal_price = 100 * (1 + clamp(0/1000, -0.05, 0.05)) = 100 * 1.0 = **100**
2. Cutoff check: limit(97) <= marginal(100) → **PASS**
3. exec_price = 100 * (1 + clamp((0 + (-30)/2)/1000, -0.05, 0.05)) = 100 * 0.985 = **98.5**
4. Price check: exec(98.5) >= limit(97) → fill
5. skew → 0 + (-30) = -30

**Step 2: Process Buy (t=t2)**

1. marginal_price = 100 * (1 + clamp(-30/1000, -0.05, 0.05)) = 100 * 0.97 = **97**
2. Cutoff check: limit(103) >= marginal(97) → **PASS**
3. exec_price = 100 * (1 + clamp((-30 + 30/2)/1000, -0.05, 0.05)) = 100 * 0.985 = **98.5**
4. Price check: exec(98.5) <= limit(103) → fill
5. skew → -30 + 30 = 0

**Result:**

- long_oi=130, short_oi=-130, skew=0
- BUY_ORDERS: [] (order removed)
- SELL_ORDERS: [] (order removed)
- Sell filled at exec_price = **98.5** (processed first due to earlier timestamp)
- Buy filled at exec_price = **98.5** (processed second)

**Compare to old two-phase behavior:**

| Algorithm         | Buy exec_price | Sell exec_price | Note                              |
| ----------------- | -------------- | --------------- | --------------------------------- |
| Old (buys first)  | 101.5          | 101.5           | Buyers get worse price            |
| New (interleaved) | 98.5           | 98.5            | Fair: older order processed first |

---

#### Test 12: Interleaved processing with multiple orders

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS:
  - Order B1: {limit_price=103, created_at=t1, size=+20, user_pos=0, reduce_only=false}
  - Order B2: {limit_price=102, created_at=t4, size=+20, user_pos=0, reduce_only=false}
- SELL_ORDERS:
  - Order S1: {limit_price=97, created_at=t2, size=-20, user_pos=0, reduce_only=false}
  - Order S2: {limit_price=98, created_at=t3, size=-20, user_pos=0, reduce_only=false}

**Trigger:** Oracle price = 100

**Processing order by timestamp: t1(B1) → t2(S1) → t3(S2) → t4(B2)**

**Step 1: Process B1 (t=t1, limit=103)**

1. marginal_price = 100, limit(103) >= marginal(100) → continue
2. exec_price = 100 * (1 + 20/2/1000) = 100 * 1.01 = **101**
3. Price check: exec(101) <= limit(103) → fill
4. skew → 0 + 20 = +20

**Step 2: Process S1 (t=t2, limit=97)**

1. marginal_price = 100 * (1 + 20/1000) = **102**
2. Cutoff check: limit(97) <= marginal(102) → **PASS**
3. exec_price = 100 * (1 + clamp((20 + (-20)/2)/1000, -0.05, 0.05)) = 100 * 1.01 = **101**
4. Price check: exec(101) >= limit(97) → fill
5. skew → 20 + (-20) = 0

**Step 3: Process S2 (t=t3, limit=98)**

1. marginal_price = 100 * (1 + 0/1000) = **100**
2. Cutoff check: limit(98) <= marginal(100) → **PASS**
3. exec_price = 100 * (1 + clamp((0 + (-20)/2)/1000, -0.05, 0.05)) = 100 * 0.99 = **99**
4. Price check: exec(99) >= limit(98) → fill
5. skew → 0 + (-20) = -20

**Step 4: Process B2 (t=t4, limit=102)**

1. marginal_price = 100 * (1 + (-20)/1000) = **98**
2. Cutoff check: limit(102) >= marginal(98) → **PASS**
3. exec_price = 100 * (1 + clamp((-20 + 20/2)/1000, -0.05, 0.05)) = 100 * 0.99 = **99**
4. Price check: exec(99) <= limit(102) → fill
5. skew → -20 + 20 = 0

**Result:**

- long_oi=140, short_oi=-140, skew=0
- All orders filled in timestamp order
- Execution prices: B1=101, S1=101, S2=99, B2=99

---

#### Test 13: Timestamp tiebreaker (buy wins)

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS: [{limit_price=103, created_at=t1, size=+20, user_pos=0, reduce_only=false}]
- SELL_ORDERS: [{limit_price=97, created_at=t1, size=-20, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Interleaved Processing (same timestamp, buy wins tiebreaker):**

**Step 1: Process Buy (t=t1, wins tiebreaker)**

1. marginal_price = 100, limit(103) >= marginal(100) → continue
2. exec_price = 100 * (1 + 20/2/1000) = 100 * 1.01 = **101**
3. Price check: exec(101) <= limit(103) → fill
4. skew → 0 + 20 = +20

**Step 2: Process Sell (t=t1)**

1. marginal_price = 100 * (1 + 20/1000) = **102**
2. Cutoff check: limit(97) <= marginal(102) → **PASS**
3. exec_price = 100 * (1 + clamp((20 + (-20)/2)/1000, -0.05, 0.05)) = 100 * 1.01 = **101**
4. Price check: exec(101) >= limit(97) → fill
5. skew → 20 + (-20) = 0

**Result:**

- long_oi=120, short_oi=-120, skew=0
- BUY_ORDERS: [] (order removed)
- SELL_ORDERS: [] (order removed)
- Buy filled at exec_price = **101** (processed first due to tiebreaker)
- Sell filled at exec_price = **101** (processed second)

---

#### Test 14: Cutoff unblocked by other side

This test verifies that when one side hits the marginal price cutoff, processing the other side can change the skew enough to make the first side fillable again.

**Initial State:**

- long_oi=150, short_oi=-100, skew=50
- BUY_ORDERS: [{limit_price=104, created_at=t1, size=+20, user_pos=0, reduce_only=false}]
- SELL_ORDERS: [{limit_price=97, created_at=t2, size=-100, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Processing (correct algorithm):**

**Iteration 1:**

1. marginal_price = 100 * (1 + clamp(50/1000, -0.05, 0.05)) = 100 * 1.05 = **105**
2. buy_fillable? limit(104) >= marginal(105)? **NO**
3. sell_fillable? limit(97) <= marginal(105)? **YES**
4. Only sell is fillable → process sell
5. exec_price = 100 * (1 + clamp((50 + (-100)/2)/1000, -0.05, 0.05)) = 100 * 1.0 = **100**
6. Price check: exec(100) >= limit(97) → fill
7. skew → 50 + (-100) = **-50**

**Iteration 2:**

1. marginal_price = 100 * (1 + clamp(-50/1000, -0.05, 0.05)) = 100 * 0.95 = **95**
2. buy_fillable? limit(104) >= marginal(95)? **YES** (now fillable!)
3. sell_fillable? No more sells
4. Only buy is fillable → process buy
5. exec_price = 100 * (1 + clamp((-50 + 20/2)/1000, -0.05, 0.05)) = 100 * 0.96 = **96**
6. Price check: exec(96) <= limit(104) → fill
7. skew → -50 + 20 = **-30**

**Iteration 3:**

1. marginal_price = 100 * (1 + clamp(-30/1000, -0.05, 0.05)) = 100 * 0.97 = **97**
2. buy_fillable? No more buys
3. sell_fillable? No more sells
4. (false, false) → **break**

**Result:**

- long_oi=170, short_oi=-200, skew=-30
- BUY_ORDERS: [] (order removed)
- SELL_ORDERS: [] (order removed)
- Sell filled at exec_price = **100**
- Buy filled at exec_price = **96**

**Compare to buggy algorithm (drain-on-cutoff):**

The buggy algorithm would:

1. Buy is older (t1) → try buy first
2. limit(104) < marginal(105) → cutoff, return false
3. Drain sells → skew=-50, marginal=95
4. Exit loop ← BUG: buy never reconsidered!

| Algorithm | Buy filled? | Sell filled? | Note                               |
| --------- | ----------- | ------------ | ---------------------------------- |
| Buggy     | **NO**      | YES          | Buy unfairly left in queue         |
| Correct   | YES         | YES          | Both filled; skew change unblocked |

### Analysis Notes

#### Note 1: Cutoff vs Continue behavior

- **break (cutoff):** When `limit_price < marginal_price` (buys) or `limit_price > marginal_price` (sells), all subsequent orders in that phase are guaranteed unfillable due to price ordering.
- **continue:** When an individual order fails the exec_price check (too large), we continue scanning because smaller orders with lower priority might still be fillable.

#### Note 2: Skew dynamics during fulfillment

Each fill changes the skew, which affects:

1. The marginal price for subsequent cutoff checks
2. The execution price for subsequent orders

For buys: filling increases skew → increases marginal price → cutoff moves up → fewer subsequent buys fillable.
For sells: filling decreases skew → decreases marginal price → cutoff moves down → fewer subsequent sells fillable.

#### Note 3: Interleaved processing

Buy orders and sell orders are processed in an interleaved manner based on timestamp. Within each side, the best-priced order is always considered (highest for buys, lowest for sells). Between the two "head" orders, the older one is processed first. This ensures neither buyers nor sellers have a systematic advantage from processing order.

When timestamps are equal, buy orders are processed first (documented tiebreaker).

### Conclusion

All test cases verify the algorithm correctly handles:

| Category                     | Behavior                                  | Tests  |
| ---------------------------- | ----------------------------------------- | ------ |
| Basic fills                  | Fill when price conditions met            | 1, 2   |
| Cutoff (marginal price)      | Break when limit vs marginal fails        | 3, 4   |
| Size-based skip              | Continue when exec > limit (buys)         | 5      |
| Price-time priority          | Highest/lowest price first, then oldest   | 6, 7   |
| Skew dynamics                | Fills affect subsequent order eligibility | 8, 14  |
| OI constraint (reduce=false) | Skip order entirely                       | 9      |
| OI constraint (reduce=true)  | Partial fill (closing only), update order | 10     |
| Interleaved processing       | Older timestamp processed first           | 11, 12 |
| Timestamp tiebreaker         | Buy wins when timestamps equal            | 13     |
| Cutoff re-evaluation         | Other side can unblock a cutoff           | 14     |

## Margin Reservation

Test cases for the margin reservation system that validates margin at order placement time.

### Test Parameters

```plain
oracle_price = 100
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
max_abs_oi = 500
initial_margin_ratio = 0.05 (5% = 20x max leverage)
```

### Key Formulas

- `used_margin = sum(|position_size| * oracle_price * initial_margin_ratio)` for all positions
- `available_margin = balance - used_margin - reserved_margin`
- `required_margin = |opening_size| * worst_case_price * initial_margin_ratio`
- For limit orders: `worst_case_price = limit_price`
- For market orders: `worst_case_price = marginal_price * (1 ± max_slippage)`

### Test Case Summary Table

| #   | Scenario                              | balance | used | reserved | available | required | Result               |
| --- | ------------------------------------- | ------- | ---- | -------- | --------- | -------- | -------------------- |
| 1   | New position, sufficient margin       | 1000    | 0    | 0        | 1000      | 500      | Order accepted       |
| 2   | New position, insufficient margin     | 100     | 0    | 0        | 100       | 500      | Order rejected       |
| 3   | Existing position uses margin         | 1000    | 500  | 0        | 500       | 500      | Order accepted       |
| 4   | Reserved margin blocks order          | 1000    | 0    | 600      | 400       | 500      | Order rejected       |
| 5   | Closing position requires no margin   | 100     | 500  | 0        | 0*        | 0        | Order accepted       |
| 6   | Partial close + opening               | 600     | 500  | 0        | 100       | 250      | Order rejected       |
| 7   | Limit order reserves margin           | 1000    | 0    | 0        | 1000      | 500      | reserved → 500       |
| 8   | Cancel order releases reserved margin | 1000    | 0    | 500      | 500       | -        | reserved → 0         |
| 9   | GTC fill releases reserved margin     | 1000    | 0    | 500      | 500       | -        | reserved → 0, used ↑ |
| 10  | Withdrawal blocked by reserved margin | 1000    | 0    | 600      | 400       | -        | Withdraw 500 fails   |
| 11  | Withdrawal blocked by used margin     | 1000    | 600  | 0        | 400       | -        | Withdraw 500 fails   |
| 12  | Withdrawal succeeds with available    | 1000    | 300  | 200      | 500       | -        | Withdraw 400 OK      |

*Note: When closing a position, used_margin is calculated on current position, but required_margin is 0 since opening_size=0.

### Detailed Calculations

#### Test 1: New position, sufficient margin

**Initial State:**

- balance=1000, used_margin=0, reserved_margin=0
- user_pos=0, long_oi=100, short_oi=-100, skew=0

**Order:** Market buy +100 contracts, max_slippage=5%

**Calculation:**

1. decompose_fill(+100, 0): closing=0, opening=+100
2. worst_case_price = marginal_price * (1 + 0.05) = 100 * 1.05 = **105**
3. required_margin = |100| * 105 * 0.05 = **525**
4. available_margin = 1000 - 0 - 0 = **1000**
5. Check: available(1000) >= required(525) → **PASS**
6. Order accepted, executes at exec_price = 102.5

**Result:**

- Order fills, position opened
- used_margin becomes 100 * 100 * 0.05 = 500 (at current oracle price)

---

#### Test 2: New position, insufficient margin

**Initial State:**

- balance=100, used_margin=0, reserved_margin=0
- user_pos=0, long_oi=100, short_oi=-100, skew=0

**Order:** Market buy +100 contracts, max_slippage=5%

**Calculation:**

1. decompose_fill(+100, 0): closing=0, opening=+100
2. worst_case_price = 100 * 1.05 = **105**
3. required_margin = |100| * 105 * 0.05 = **525**
4. available_margin = 100 - 0 - 0 = **100**
5. Check: available(100) >= required(525) → **FAIL**

**Result:**

- Order rejected with "insufficient margin"
- No state changes

---

#### Test 3: Existing position uses margin

**Initial State:**

- balance=1000, user_pos=+100 (existing long)
- used_margin = 100 * 100 * 0.05 = **500**
- reserved_margin=0

**Order:** Market buy +100 contracts (increase long), max_slippage=5%

**Calculation:**

1. decompose_fill(+100, +100): closing=0, opening=+100
2. worst_case_price = 100 * 1.05 = **105**
3. required_margin = |100| * 105 * 0.05 = **525**
4. available_margin = 1000 - 500 - 0 = **500**
5. Check: available(500) >= required(525) → **FAIL**

**Result:**

- Order rejected with "insufficient margin"

**Alternative:** With balance=1100:

- available = 1100 - 500 - 0 = 600 >= 525 → Order accepted

---

#### Test 4: Reserved margin blocks order

**Initial State:**

- balance=1000, used_margin=0
- reserved_margin=600 (from pending limit order)
- user_pos=0

**Order:** Market buy +100 contracts, max_slippage=5%

**Calculation:**

1. decompose_fill(+100, 0): closing=0, opening=+100
2. required_margin = 100 * 105 * 0.05 = **525**
3. available_margin = 1000 - 0 - 600 = **400**
4. Check: available(400) >= required(525) → **FAIL**

**Result:**

- Order rejected with "insufficient margin"
- User must cancel pending order to free up reserved margin

---

#### Test 5: Closing position requires no margin

**Initial State:**

- balance=100, user_pos=+100 (long position)
- used_margin = 100 * 100 * 0.05 = **500**
- reserved_margin=0
- (Note: balance < used_margin is possible if position has unrealized loss)

**Order:** Market sell -100 contracts (close entire long)

**Calculation:**

1. decompose_fill(-100, +100): closing=-100, opening=0
2. opening_size = 0, so required_margin = **0**
3. available_margin = 100 - 500 - 0 = **0** (saturating_sub)
4. Check: available(0) >= required(0) → **PASS**

**Result:**

- Order accepted, position closed
- used_margin → 0 (no position)
- Balance may change based on realized PnL

---

#### Test 6: Partial close + opening (flip position)

**Initial State:**

- balance=600, user_pos=+100 (long position)
- used_margin = 100 * 100 * 0.05 = **500**
- reserved_margin=0

**Order:** Market sell -150 contracts (close long, open short)

**Calculation:**

1. decompose_fill(-150, +100): closing=-100, opening=-50
2. worst_case_price (for sell) = marginal_price * (1 - slippage) = 100 * 0.95 = **95**
3. required_margin = |-50| * 95 * 0.05 = **237.5** → ceil = **238**
4. available_margin = 600 - 500 - 0 = **100**
5. Check: available(100) >= required(238) → **FAIL**

**Result:**

- Order rejected with "insufficient margin"
- User can submit with reduce_only=true to just close the long

---

#### Test 7: Limit order reserves margin

**Initial State:**

- balance=1000, used_margin=0, reserved_margin=0
- user_pos=0, skew=0

**Order:** Limit buy +100 contracts at limit_price=105

**Calculation:**

1. decompose_fill(+100, 0): closing=0, opening=+100
2. worst_case_price = limit_price = **105**
3. required_margin = 100 * 105 * 0.05 = **525**
4. available_margin = 1000 - 0 - 0 = **1000**
5. Check: available(1000) >= required(525) → **PASS**
6. Price check: exec_price(102.5) <= limit_price(105) → fill immediately
7. No unfilled portion → no margin to reserve

**Alternative:** Limit buy at limit_price=99 (below marginal):

- Price check fails, entire order goes to GTC storage
- reserved_margin += 100 * 99 * 0.05 = **495**

**Result (limit=99):**

- reserved_margin = 0 → **495**
- Order stored in BUY_ORDERS for later fulfillment

---

#### Test 8: Cancel order releases reserved margin

**Initial State:**

- balance=1000, used_margin=0, reserved_margin=500
- Pending buy order: size=+100, limit_price=100

**Action:** Cancel order

**Calculation:**

1. Load order, verify ownership
2. Compute reserved margin for order:
   - decompose_fill(+100, user_pos): opening=+100
   - reserved = 100 * 100 * 0.05 = **500**
3. user_state.reserved_margin -= 500

**Result:**

- reserved_margin = 500 → **0**
- available_margin = 1000 - 0 - 0 = **1000**

---

#### Test 9: GTC fill releases reserved margin

**Initial State:**

- balance=1000, used_margin=0, reserved_margin=500
- Pending buy order in storage: size=+100, limit_price=105, user_pos=0

**Trigger:** Oracle price drops, order becomes fillable

**Calculation:**

1. Order fills at exec_price (e.g., 100)
2. Position created: user_pos = +100
3. Release reserved margin:
   - opening_size = +100
   - reserved_for_order = 100 * 105 * 0.05 = **525** (but we only reserved 500, use stored value)
   - user_state.reserved_margin -= 500
4. Position now uses margin:
   - used_margin = 100 * 100 * 0.05 = **500** (at current oracle price)

**Result:**

- reserved_margin = 500 → **0**
- used_margin = 0 → **500**
- available_margin = 1000 - 500 - 0 = **500**

---

#### Test 10: Withdrawal blocked by reserved margin

**Initial State:**

- balance=1000, used_margin=0, reserved_margin=600

**Action:** Withdraw 500 USDT

**Calculation:**

1. available_margin = 1000 - 0 - 600 = **400**
2. Check: amount(500) <= available(400) → **FAIL**

**Result:**

- Withdrawal rejected with "insufficient available margin"
- User must cancel pending orders to free up reserved margin

---

#### Test 11: Withdrawal blocked by used margin

**Initial State:**

- balance=1000, used_margin=600, reserved_margin=0

**Action:** Withdraw 500 USDT

**Calculation:**

1. available_margin = 1000 - 600 - 0 = **400**
2. Check: amount(500) <= available(400) → **FAIL**

**Result:**

- Withdrawal rejected with "insufficient available margin"
- User must close positions to free up used margin

---

#### Test 12: Withdrawal succeeds with available margin

**Initial State:**

- balance=1000, used_margin=300, reserved_margin=200

**Action:** Withdraw 400 USDT

**Calculation:**

1. available_margin = 1000 - 300 - 200 = **500**
2. Check: amount(400) <= available(500) → **PASS**
3. balance -= 400

**Result:**

- balance = 1000 → **600**
- available_margin = 600 - 300 - 200 = **100**

### Analysis Notes

#### Note 1: LP vs Trader separation

The margin system only applies to **trading balance** (`user_state.balance`), not vault shares. These are completely separate:

- **Vault shares** (`user_state.vault_shares`): LP activity, counterparty risk
- **Trading balance** (`user_state.balance`): Trader collateral for positions

A user can deposit 1000 USDT to vault (LP) and separately deposit 1000 USDT to trading balance (trader). The two pools don't interact.

#### Note 2: Worst-case price rationale

For margin reservation, we use the worst-case execution price:

- **Buy orders**: Limit price (maximum the user will pay)
- **Sell orders**: Limit price (minimum the user will receive)
- **Market orders**: Marginal price ± slippage

This ensures we reserve enough margin even if the actual execution price is worse than expected.

#### Note 3: Closing portion releases margin

When a position is closed:

1. The closing portion requires no new margin (opening_size = 0)
2. The used_margin decreases as position size decreases
3. This frees up margin for other activities

This is why users can always close positions even with low balance—closing never requires additional margin.

### Conclusion

All test cases verify the margin reservation system correctly handles:

| Category                  | Behavior                              | Tests |
| ------------------------- | ------------------------------------- | ----- |
| New position margin check | Reject if available < required        | 1, 2  |
| Existing position margin  | Used margin reduces available         | 3     |
| Reserved margin blocks    | Reserved margin reduces available     | 4     |
| Closing needs no margin   | Opening_size=0 means required=0       | 5     |
| Flip position margin      | Closing releases, opening requires    | 6     |
| Limit order reservation   | Unfilled portion reserves margin      | 7     |
| Cancel releases margin    | Reserved margin returned to available | 8     |
| Fill releases reservation | Reserved → used margin on fill        | 9     |
| Withdrawal restrictions   | Cannot withdraw more than available   | 10-12 |

## Vault Unrealized PnL

Test cases for the `oi_weighted_entry_price` accumulator and `compute_vault_unrealized_pnl` function.

### Test Parameters

```plain
Pair: BTCUSD
oracle_price = 50,000
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
```

### Key Formulas

- `oi_weighted_entry_price = Σ sign(pos.size) * pos.cost_basis`
- `skew = long_oi + short_oi`
- `vault_unrealized_pnl = oi_weighted_entry_price - oracle_price * skew`

### Test 1: Multi-position vault unrealized PnL

Four positions covering all quadrants (long/short × profit/loss from the vault's perspective):

| #   | User  | size | entry_price | cost_basis                  | sign(size) * cost_basis |
| --- | ----- | ---- | ----------- | --------------------------- | ----------------------- |
| 1   | Alice | +2   | 48,000      | floor(2 * 48,000) = 96,000  | +96,000                 |
| 2   | Bob   | -3   | 52,000      | floor(3 * 52,000) = 156,000 | -156,000                |
| 3   | Carol | +1   | 51,000      | floor(1 * 51,000) = 51,000  | +51,000                 |
| 4   | Dave  | -1   | 49,000      | floor(1 * 49,000) = 49,000  | -49,000                 |

**Step 1: Compute `oi_weighted_entry_price`**

```plain
oi_weighted_entry_price = 96,000 + (-156,000) + 51,000 + (-49,000)
                        = -58,000
```

**Step 2: Compute OI and skew**

```plain
long_oi  = 2 + 1 = +3
short_oi = -3 + (-1) = -4
skew     = 3 + (-4) = -1
```

**Step 3: Compute vault unrealized PnL**

```plain
vault_unrealized_pnl = oi_weighted_entry_price - oracle_price * skew
                     = -58,000 - 50,000 * (-1)
                     = -58,000 + 50,000
                     = -8,000
```

**Step 4: Verify against per-position manual calculation**

The vault is the counterparty to all traders. For each trader's position, the vault holds the opposite. The vault's unrealized PnL per position is the negative of the trader's unrealized PnL:

| #   | Trader PnL formula                                 | Trader PnL | Vault PnL |
| --- | -------------------------------------------------- | ---------- | --------- |
| 1   | (50,000 - 48,000) * 2 = +4,000 (long, price up)    | +4,000     | -4,000    |
| 2   | (52,000 - 50,000) * 3 = +6,000 (short, price down) | +6,000     | -6,000    |
| 3   | (50,000 - 51,000) * 1 = -1,000 (long, price down)  | -1,000     | +1,000    |
| 4   | (49,000 - 50,000) * 1 = -1,000 (short, price up)   | -1,000     | +1,000    |

```plain
vault_unrealized_pnl = -4,000 + (-6,000) + 1,000 + 1,000 = -8,000 ✓
```

The accumulator-based computation matches the per-position calculation.

### Test 2: Accumulator maintenance through fills

This test verifies that `oi_weighted_entry_price` is correctly maintained through a sequence of fills: open, partial close, and flip.

**Parameters:**

```plain
Pair: BTCUSD
oracle_price = 50,000 (constant throughout; skew pricing ignored for clarity)
```

**Step 1: Alice opens long +4 at entry_price = 50,000**

```plain
cost_basis = floor(4 * 50,000) = 200,000
sign(size) = +1
```

Accumulator update (new position branch):

```plain
oi_weighted_entry_price += sign(+4) * 200,000 = +200,000

oi_weighted_entry_price = 200,000
long_oi = 4, short_oi = 0, skew = 4
vault_unrealized_pnl = 200,000 - 50,000 * 4 = 0 ✓ (just opened at current price)
```

**Step 2: Bob opens short -2 at entry_price = 50,000**

```plain
cost_basis = floor(2 * 50,000) = 100,000
sign(size) = -1
```

Accumulator update (new position branch):

```plain
oi_weighted_entry_price += sign(-2) * 100,000 = -100,000

oi_weighted_entry_price = 200,000 + (-100,000) = 100,000
long_oi = 4, short_oi = -2, skew = 2
vault_unrealized_pnl = 100,000 - 50,000 * 2 = 0 ✓
```

**Step 3: Alice partial-closes -2 at exec_price = 52,000** (oracle still 50,000)

Before modification:

```plain
Alice: size = +4, cost_basis = 200,000
Remove old contribution: oi_weighted_entry_price -= sign(+4) * 200,000 = -200,000
oi_weighted_entry_price = 100,000 - 200,000 = -100,000
```

Close portion updates cost_basis:

```plain
close_ratio = 2 / 4 = 0.5
cost_basis = floor(200,000 * (1 - 0.5)) = 100,000
```

After modification (position not fully closed):

```plain
Alice: size = +4 + (-2) = +2, cost_basis = 100,000
Add new contribution: oi_weighted_entry_price += sign(+2) * 100,000 = +100,000
oi_weighted_entry_price = -100,000 + 100,000 = 0
```

Verify:

```plain
long_oi = 4 - 2 = 2, short_oi = -2, skew = 0
vault_unrealized_pnl = 0 - 50,000 * 0 = 0
```

Per-position check:

```plain
Alice: (50,000 - 50,000) * 2 = 0 (remaining long at entry 50,000)
Bob:   (50,000 - 50,000) * 2 = 0 (short at entry 50,000)
Total trader PnL = 0, vault PnL = 0 ✓
```

Note: Alice realized PnL of (52,000 - 50,000) * 2 = 4,000 on the closed portion, which was settled to margins. The vault's _unrealized_ PnL only reflects open positions.

**Step 4: Alice flips to short -3 (fill_size = -5) at exec_price = 51,000** (oracle still 50,000)

Before modification:

```plain
Alice: size = +2, cost_basis = 100,000
Remove old contribution: oi_weighted_entry_price -= sign(+2) * 100,000 = -100,000
oi_weighted_entry_price = 0 - 100,000 = -100,000
```

Decompose fill(-5, +2): closing = -2, opening = -3

Close the remaining long:

```plain
close_ratio = 2 / 2 = 1.0
cost_basis = floor(100,000 * (1 - 1.0)) = 0
```

Open new short:

```plain
cost_basis = 0 + floor(3 * 51,000) = 153,000
```

After modification:

```plain
Alice: size = +2 + (-5) = -3, cost_basis = 153,000
Add new contribution: oi_weighted_entry_price += sign(-3) * 153,000 = -153,000
oi_weighted_entry_price = -100,000 + (-153,000) = -253,000
```

Verify:

```plain
long_oi = 2 - 2 = 0, short_oi = -2 + (-3) = -5, skew = -5
vault_unrealized_pnl = -253,000 - 50,000 * (-5) = -253,000 + 250,000 = -3,000
```

Per-position check:

```plain
Alice: short -3 at entry 51,000 → (51,000 - 50,000) * 3 = +3,000 trader PnL
Bob:   short -2 at entry 50,000 → (50,000 - 50,000) * 2 = 0 trader PnL
Total trader PnL = +3,000, vault PnL = -3,000 ✓
```

### Analysis Notes

#### Note 1: Ordering of accumulator removal

The removal of the old `oi_weighted_entry_price` contribution **must** happen in the first block of `execute_fill` (before `cost_basis` is modified for the closing portion), not in the second block. This is different from `oi_weighted_entry_funding`, where the old contribution is removed in the second block (since `entry_funding_per_unit` is updated by `settle_funding`, not by the closing logic).

#### Note 2: Vault unrealized PnL sign convention

The formula `vault_unrealized_pnl = oi_weighted_entry_price - oracle_price * skew` gives a positive result when the vault is in profit (traders are losing in aggregate) and a negative result when the vault is in loss (traders are winning).

### Conclusion

| Category                   | Behavior                                           | Tests |
| -------------------------- | -------------------------------------------------- | ----- |
| Multi-position computation | Accumulator matches per-position sum               | 1     |
| Open position              | New contribution added correctly                   | 2.1-2 |
| Partial close              | Old removed before cost_basis change, new re-added | 2.3   |
| Flip position              | Close + open handled in single fill                | 2.4   |
