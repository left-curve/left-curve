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
| 11  | Mixed buy and sell orders      | Both processed; buy phase then sell phase          |

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

#### Test 11: Mixed buy and sell orders

**Initial State:**

- long_oi=100, short_oi=-100, skew=0
- BUY_ORDERS: [{limit_price=103, size=+30, user_pos=0, reduce_only=false}]
- SELL_ORDERS: [{limit_price=97, size=-30, user_pos=0, reduce_only=false}]

**Trigger:** Oracle price = 100

**Buy Phase:**

1. marginal_price = 100, limit(103) >= marginal(100) → continue
2. exec_price = 100 * (1 + 30/2/1000) = 100 * 1.015 = **101.5**
3. Price check: exec(101.5) <= limit(103) → fill
4. skew → 0+30 = 30

**Sell Phase (with updated skew=30):**

1. marginal_price = 100 * (1 + 30/1000) = 100 * 1.03 = **103**
2. Cutoff check: limit(97) <= marginal(103) → **PASS**
3. exec_price = 100 * (1 + clamp((30-30/2)/1000, -0.05, 0.05)) = 100 * 1.015 = **101.5**
4. Price check: exec(101.5) >= limit(97) → fill
5. skew → 30-30 = 0

**Result:**

- long_oi=130, short_oi=-130, skew=0
- BUY_ORDERS: [] (order removed)
- SELL_ORDERS: [] (order removed)
- Buy filled at exec_price = **101.5**
- Sell filled at exec_price = **101.5**

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

#### Note 3: Two-phase processing

Buy orders and sell orders are processed in separate phases. The skew after the buy phase becomes the starting skew for the sell phase, which can significantly affect sell order execution prices.

### Conclusion

All test cases verify the algorithm correctly handles:

| Category                     | Behavior                                  | Tests |
| ---------------------------- | ----------------------------------------- | ----- |
| Basic fills                  | Fill when price conditions met            | 1, 2  |
| Cutoff (marginal price)      | Break when limit vs marginal fails        | 3, 4  |
| Size-based skip              | Continue when exec > limit (buys)         | 5     |
| Price-time priority          | Highest/lowest price first, then oldest   | 6, 7  |
| Skew dynamics                | Fills affect subsequent order eligibility | 8     |
| OI constraint (reduce=false) | Skip order entirely                       | 9     |
| OI constraint (reduce=true)  | Partial fill (closing only), update order | 10    |
| Mixed orders                 | Both buy and sell phases execute          | 11    |
