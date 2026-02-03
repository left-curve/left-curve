# Test Cases for SubmitOrder Algorithm

## Test Parameters

```plain
oracle_price = 100
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
max_abs_oi = 500
max_abs_skew = 300
```

## Key Formulas

- `marginal_premium = clamp(skew/K, -M, M)`
- `marginal_price = oracle_price * (1 + marginal_premium)`
- For buy: `target_price = marginal_price * (1 + max_slippage)`
- For sell: `target_price = marginal_price * (1 - max_slippage)`
- `exec_price = oracle_price * (1 + clamp((skew + fill/2)/K, -M, M))`
- **Price check (all-or-nothing):** For buy: `exec_price <= target_price`; For sell: `exec_price >= target_price`

## Test Case Summary Table

| #   | Scenario                   | user_pos | long_oi | short_oi | skew | size | kind         | fill     | exec_price | Limiting        |
| --- | -------------------------- | -------- | ------- | -------- | ---- | ---- | ------------ | -------- | ---------- | --------------- |
| 1   | New long, unconstrained    | 0        | 100     | -100     | 0    | +50  | Market(5%)   | **+50**  | 102.5      | None            |
| 2   | New short, unconstrained   | 0        | 100     | -100     | 0    | -50  | Market(5%)   | **-50**  | 97.5       | None            |
| 3   | Add long, skew over limit  | 0        | 480     | -100     | 380  | +50  | Market(10%)  | **0**    | -          | Skew (>max)     |
| 4   | Add short, skew over limit | 0        | 100     | -480     | -380 | -50  | Market(10%)  | **0**    | -          | Skew (>max)     |
| 5   | Add long, skew-limited     | 0        | 200     | -50      | 150  | +200 | Market(10%)  | **+150** | 105        | Skew (room=150) |
| 6   | Add short, skew-limited    | 0        | 50      | -200     | -150 | -200 | Market(10%)  | **-150** | 95         | Skew (room=150) |
| 7   | Close long fully           | +100     | 200     | -100     | 100  | -100 | Market(1%)   | **-100** | 105        | None (closing)  |
| 8   | Close short fully          | -100     | 100     | -200     | -100 | +100 | Market(1%)   | **+100** | 95         | None (closing)  |
| 9   | Flip long→short            | +100     | 200     | -100     | 100  | -150 | Market(5%)   | **-150** | 102.5      | None            |
| 10  | Flip, opening limited      | +100     | 200     | -480     | -280 | -150 | Market(5%)   | **-120** | 95         | OI (room=20)    |
| 11  | Buy, slippage exceeded     | 0        | 100     | -100     | 0    | +100 | Market(1%)   | **0**    | -          | Price           |
| 12  | Sell, slippage exceeded    | 0        | 100     | -100     | 0    | -100 | Market(1%)   | **0**    | -          | Price           |
| 13  | Limit buy, price exceeded  | 0        | 100     | -100     | 0    | +50  | Limit(101.5) | **0**    | -          | Limit price     |
| 14  | Limit buy, below marginal  | 0        | 100     | -100     | 0    | +50  | Limit(99)    | **0**    | -          | Limit<marginal  |
| 15  | Limit sell, price exceeded | 0        | 100     | -100     | 0    | -50  | Limit(98.5)  | **0**    | -          | Limit price     |
| 16  | Limit sell, above marginal | 0        | 100     | -100     | 0    | -50  | Limit(101)   | **0**    | -          | Limit>marginal  |
| 17  | Close at max OI            | +100     | 500     | -100     | 400  | -100 | Market(5%)   | **-100** | 105        | None (closing)  |
| 18  | Close at max skew          | +50      | 350     | -50      | 300  | -50  | Market(5%)   | **-50**  | 105        | None (closing)  |

## Detailed Calculations

### Test 1: New long, unconstrained

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, slippage=5%

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** room = 500-100 = 400, result = min(50, 400) = **+50**
3. **compute_max_opening_from_skew(+50, skew_after_close=0):** result = min(50, 300-0) = **+50**
4. **fill_size from OI/skew:** max(0, min(50, 50)) = **+50**
5. **compute_target_price:**
   - marginal_premium = clamp(0/1000, -0.05, 0.05) = 0
   - marginal_price = 100 * 1.0 = 100
   - target_price = 100 * 1.05 = 105
6. **compute_exec_price(+50, skew=0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) <= target_price (105) → **PASS**
8. **fill_size = +50** ✓

### Test 2: New short, unconstrained

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=-50, slippage=5%

1. **decompose_fill(-50, 0):** closing=0, opening=-50
2. **compute_max_opening_from_oi(-50):** room = 500-100 = 400, result = max(-50, -400) = **-50**
3. **compute_max_opening_from_skew(-50, 0):** result = max(-50, -300-0) = max(-50, -300) = **-50**
4. **fill_size from OI/skew:** min(0, max(-50, -50)) = **-50**
5. **compute_target_price:**
   - marginal_price = 100
   - target_price = 100 * 0.95 = 95
6. **compute_exec_price(-50, skew=0):**
   - premium = clamp((0 + (-50)/2) / 1000, -0.05, 0.05) = -0.025
   - exec_price = 100 * 0.975 = **97.5**
7. **Price check:** exec_price (97.5) >= target_price (95) → **PASS**
8. **fill_size = -50** ✓

### Test 3: Add long, skew over limit

**Input:** user_pos=0, long_oi=480, short_oi=-100, skew=380, size=+50, slippage=10%

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** room = 500-480 = 20, result = min(50, 20) = **+20**
3. **compute_max_opening_from_skew(+50, 380):** result = min(50, 300-380) = min(50, -80) = **-80**
   - This returns -80 because skew (380) already exceeds max_abs_skew (300)
4. **fill_size from OI/skew:** max(0, min(50, -80)) = max(0, -80) = **0**
5. **fill_size = 0** ✓ (no price check needed since fill is already 0)

**Analysis:** Correct behavior. Skew is already violated, so no new longs allowed.

### Test 5: Add long, skew-limited

**Input:** user_pos=0, long_oi=200, short_oi=-50, skew=150, size=+200, slippage=10%

1. **decompose_fill(+200, 0):** closing=0, opening=+200
2. **compute_max_opening_from_oi(+200):** room = 500-200 = 300, result = min(200, 300) = **+200**
3. **compute_max_opening_from_skew(+200, 150):** result = min(200, 300-150) = min(200, 150) = **+150**
4. **fill_size from OI/skew:** max(0, min(200, 150)) = **+150**
5. **compute_target_price:**
   - marginal_premium = clamp(150/1000, -0.05, 0.05) = 0.05 (clamped)
   - marginal_price = 100 * 1.05 = 105
   - target_price = 105 * 1.10 = 115.5
6. **compute_exec_price(+150, skew=150):**
   - premium = clamp((150 + 150/2) / 1000, -0.05, 0.05) = clamp(0.225, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) <= target_price (115.5) → **PASS**
8. **fill_size = +150** ✓

### Test 7: Close long fully (closing always allowed)

**Input:** user_pos=+100, long_oi=200, short_oi=-100, skew=100, size=-100, slippage=1%

1. **decompose_fill(-100, +100):**
   - size < 0, user_pos > 0 → closing = max(-100, -100) = -100, opening = 0
2. **compute_max_opening_from_oi(0):** returns **0**
3. **compute_max_opening_from_skew(0, skew_after_close=0):** returns **0**
4. **fill_size from OI/skew:** min(0, max(-100, -100)) = **-100**
5. **compute_target_price:**
   - marginal_premium = clamp(100/1000, -0.05, 0.05) = 0.05 (clamped)
   - marginal_price = 100 * 1.05 = 105
   - target_price = 105 * 0.99 = 103.95
6. **compute_exec_price(-100, skew=100):**
   - premium = clamp((100 + (-100)/2) / 1000, -0.05, 0.05) = clamp(0.05, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) >= target_price (103.95) → **PASS**
8. **fill_size = -100** ✓

### Test 10: Flip, opening limited by OI

**Input:** user_pos=+100, long_oi=200, short_oi=-480, skew=-280, size=-150, slippage=5%

1. **decompose_fill(-150, +100):**
   - closing = max(-150, -100) = -100, opening = -150 - (-100) = -50
2. **compute_max_opening_from_oi(-50):**
   - short_oi.abs() = 480, room = 500 - 480 = 20
   - result = max(-50, -20) = **-20** (limited!)
3. **compute_max_opening_from_skew(-50, skew_after_close=-380):**
   - skew_after_close = -280 + (-100) = -380
   - result = max(-50, -300 - (-380)) = max(-50, 80) = **80**
   - But since opening is negative, this means no skew constraint
4. **fill_size from OI/skew:** -100 + (-20) = **-120**
5. **compute_target_price:**
   - marginal_premium = clamp(-280/1000, -0.05, 0.05) = -0.05 (clamped)
   - marginal_price = 100 * 0.95 = 95
   - target_price = 95 * 0.95 = 90.25
6. **compute_exec_price(-120, skew=-280):**
   - premium = clamp((-280 + (-120)/2) / 1000, -0.05, 0.05) = clamp(-0.34, -0.05, 0.05) = -0.05
   - exec_price = 100 * 0.95 = **95**
7. **Price check:** exec_price (95) >= target_price (90.25) → **PASS**
8. **fill_size = -120** ✓

### Test 11: Buy, slippage exceeded (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+100, slippage=1%

1. **decompose_fill(+100, 0):** closing=0, opening=+100
2. **compute_max_opening_from_oi(+100):** room = 400, result = **+100**
3. **compute_max_opening_from_skew(+100, 0):** result = min(100, 300) = **+100**
4. **fill_size from OI/skew:** max(0, min(100, 100)) = **+100**
5. **compute_target_price:**
   - marginal_premium = 0
   - marginal_price = 100
   - target_price = 100 * 1.01 = 101
6. **compute_exec_price(+100, 0):**
   - premium = clamp((0 + 100/2) / 1000, -0.05, 0.05) = 0.05
   - exec_price = 100 * 1.05 = **105**
7. **Price check:** exec_price (105) > target_price (101) → **FAIL**
8. **fill_size = 0** ✓ (order rejected due to slippage)

### Test 13: Limit buy, price exceeded (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, limit_price=101.5

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** result = **+50**
3. **compute_max_opening_from_skew(+50, 0):** result = **+50**
4. **fill_size from OI/skew:** max(0, min(50, 50)) = **+50**
5. **compute_target_price:** target_price = 101.5
6. **compute_exec_price(+50, 0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) > target_price (101.5) → **FAIL**
8. **fill_size = 0** ✓ (order stored as GTC for later fulfillment)

### Test 14: Limit buy, below marginal (all-or-nothing)

**Input:** user_pos=0, long_oi=100, short_oi=-100, skew=0, size=+50, limit_price=99

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** result = **+50**
3. **compute_max_opening_from_skew(+50, 0):** result = **+50**
4. **fill_size from OI/skew:** max(0, min(50, 50)) = **+50**
5. **compute_target_price:** target_price = 99
6. **compute_exec_price(+50, 0):**
   - premium = clamp((0 + 50/2) / 1000, -0.05, 0.05) = 0.025
   - exec_price = 100 * 1.025 = **102.5**
7. **Price check:** exec_price (102.5) > target_price (99) → **FAIL**
8. **fill_size = 0** ✓ (order stored as GTC for later fulfillment)

## Analysis Notes

### Note 1: Skew constraint when already over limit

When `skew > max_abs_skew` and user tries to open a long, the formula `min(opening_size, max_abs_skew - skew)` returns a negative number. This is intentional and works correctly because:

1. The negative value propagates through `max_opening = min(max_oi, max_skew)`
2. This results in a negative `max_from_oi_skew`
3. The final `fill_size = max(0, ...)` clamp ensures fill = 0

**Result:** Correct behavior - no new positions allowed that would increase an already-violated skew.

### Note 2: Closing always allowed

The decomposition into closing/opening portions ensures that closing an existing position is never blocked by OI/skew constraints, even when those constraints are at their limits.

## Conclusion

All test cases verify the algorithm correctly handles:

| Category                 | Behavior                         | Tests        |
| ------------------------ | -------------------------------- | ------------ |
| Unconstrained orders     | Full fill                        | 1, 2         |
| Skew over limit          | Fill = 0                         | 3, 4         |
| Skew-limited             | Partial fill                     | 5, 6         |
| Closing positions        | Always allowed                   | 7, 8, 17, 18 |
| Position flips           | Close + constrained open         | 9, 10        |
| Slippage exceeded        | Reject (all-or-nothing)          | 11, 12       |
| Limit price exceeded     | Fill = 0, stored as GTC          | 13, 15       |
| Limit below/above market | Fill = 0, stored as GTC          | 14, 16       |
