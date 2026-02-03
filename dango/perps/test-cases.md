# Test Cases for SubmitOrder Algorithm

## Test Parameters

```plain
oracle_price = 100
K (skew_scale) = 1000
M (max_abs_premium) = 0.05 (5%)
max_abs_oi = 500
```

## Key Formulas

- `marginal_premium = clamp(skew/K, -M, M)`
- `marginal_price = oracle_price * (1 + marginal_premium)`
- For buy: `target_price = marginal_price * (1 + max_slippage)`
- For sell: `target_price = marginal_price * (1 - max_slippage)`
- `exec_price = oracle_price * (1 + clamp((skew + fill/2)/K, -M, M))`
- **Price check (all-or-nothing):** For buy: `exec_price <= target_price`; For sell: `exec_price >= target_price`

## Test Case Summary Table

| #   | Scenario                          | user_pos | long_oi | short_oi | skew | size | reduce_only | kind         | fill     | exec_price | Limiting         |
| --- | --------------------------------- | -------- | ------- | -------- | ---- | ---- | ----------- | ------------ | -------- | ---------- | ---------------- |
| 1   | New long, unconstrained           | 0        | 100     | -100     | 0    | +50  | false       | Market(5%)   | **+50**  | 102.5      | None             |
| 2   | New short, unconstrained          | 0        | 100     | -100     | 0    | -50  | false       | Market(5%)   | **-50**  | 97.5       | None             |
| 3   | New long, OI blocked              | 0        | 480     | -100     | 380  | +50  | false       | Market(5%)   | **0**    | -          | OI (revert)      |
| 4   | New short, OI blocked             | 0        | 100     | -480     | -380 | -50  | false       | Market(5%)   | **0**    | -          | OI (revert)      |
| 5   | Close long fully                  | +100     | 200     | -100     | 100  | -100 | false       | Market(1%)   | **-100** | 105        | None (closing)   |
| 6   | Close short fully                 | -100     | 100     | -200     | -100 | +100 | false       | Market(1%)   | **+100** | 95         | None (closing)   |
| 7   | Flip, reduce_only=false, OK       | +100     | 200     | -100     | 100  | -150 | false       | Market(5%)   | **-150** | 102.5      | None             |
| 8   | Flip, reduce_only=false, OI block | +100     | 200     | -480     | -280 | -150 | false       | Market(5%)   | **0**    | -          | OI (revert all)  |
| 9   | Flip, reduce_only=true            | +100     | 200     | -480     | -280 | -150 | true        | Market(5%)   | **-100** | 95         | reduce_only      |
| 10  | Slippage exceeded                 | 0        | 100     | -100     | 0    | +100 | false       | Market(1%)   | **0**    | -          | Price            |
| 11  | Limit buy, price exceeded         | 0        | 100     | -100     | 0    | +50  | false       | Limit(101.5) | **0**    | -          | Limit price      |
| 12  | Limit buy, below marginal         | 0        | 100     | -100     | 0    | +50  | false       | Limit(99)    | **0**    | -          | Limit<marginal   |
| 13  | Close at max OI                   | +100     | 500     | -100     | 400  | -100 | false       | Market(5%)   | **-100** | 105        | None (closing)   |

## Detailed Calculations

### Test 1: New long, unconstrained

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

### Test 2: New short, unconstrained

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

### Test 3: New long, OI blocked

**Input:** user_pos=0, long_oi=480, short_oi=-100, skew=380, size=+50, reduce_only=false, slippage=5%

1. **decompose_fill(+50, 0):** closing=0, opening=+50
2. **compute_max_opening_from_oi(+50):** room = 500-480 = 20, result = min(50, 20) = **+20**
3. **OI violated?** max_opening (20) < opening (50)? Yes → OI violated
4. **reduce_only=false:** entire order reverts
5. **fill_size = 0** ✓

### Test 4: New short, OI blocked

**Input:** user_pos=0, long_oi=100, short_oi=-480, skew=-380, size=-50, reduce_only=false, slippage=5%

1. **decompose_fill(-50, 0):** closing=0, opening=-50
2. **compute_max_opening_from_oi(-50):** room = 500-480 = 20, result = max(-50, -20) = **-20**
3. **OI violated?** max_opening (-20) > opening (-50)? Yes → OI violated
4. **reduce_only=false:** entire order reverts
5. **fill_size = 0** ✓

### Test 5: Close long fully (closing always allowed)

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

### Test 6: Close short fully (closing always allowed)

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

### Test 7: Flip long→short, reduce_only=false, OI OK

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

### Test 8: Flip, reduce_only=false, OI blocked

**Input:** user_pos=+100, long_oi=200, short_oi=-480, skew=-280, size=-150, reduce_only=false, slippage=5%

1. **decompose_fill(-150, +100):**
   - closing = max(-150, -100) = -100, opening = -150 - (-100) = -50
2. **compute_max_opening_from_oi(-50):**
   - short_oi.abs() = 480, room = 500 - 480 = 20
   - result = max(-50, -20) = **-20** (limited!)
3. **OI violated?** max_opening (-20) > opening (-50)? Yes → OI violated
4. **reduce_only=false:** entire order (including closing) reverts
5. **fill_size = 0** ✓

### Test 9: Flip, reduce_only=true

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

### Test 10: Buy, slippage exceeded (all-or-nothing)

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

### Test 11: Limit buy, price exceeded (all-or-nothing)

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

### Test 12: Limit buy, below marginal (all-or-nothing)

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

### Test 13: Close at max OI (closing always allowed)

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

## Analysis Notes

### Note 1: reduce_only behavior

When `reduce_only = true`:

- The closing portion is always executed
- The opening portion is discarded (not executed)
- This allows users to close positions even when OI constraints would block new positions

When `reduce_only = false`:

- If opening would violate OI, the entire order reverts (all-or-nothing)
- This ensures users get their full intended exposure or nothing

### Note 2: Closing always allowed

The decomposition into closing/opening portions ensures that closing an existing position is never blocked by OI constraints, even when those constraints are at their limits. This is important because:

1. Closing reduces risk in the system
2. Users should always be able to exit positions

## Conclusion

All test cases verify the algorithm correctly handles:

| Category                | Behavior                            | Tests        |
| ----------------------- | ----------------------------------- | ------------ |
| Unconstrained orders    | Full fill                           | 1, 2         |
| OI blocked (opening)    | Revert if reduce_only=false         | 3, 4         |
| Closing positions       | Always allowed                      | 5, 6, 13     |
| Flip, OI OK             | Full fill                           | 7            |
| Flip, OI blocked        | Revert all if reduce_only=false     | 8            |
| Flip, reduce_only=true  | Close only, discard opening         | 9            |
| Slippage exceeded       | Reject (all-or-nothing)             | 10           |
| Limit price exceeded    | Fill = 0, stored as GTC             | 11           |
| Limit below market      | Fill = 0, stored as GTC             | 12           |
