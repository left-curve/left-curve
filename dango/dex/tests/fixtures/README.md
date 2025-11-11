# Deterministic Volatility Estimation Test Fixtures

This directory contains deterministic test fixtures for the volatility estimation algorithm used in the DEX.

## Overview

The volatility estimator uses an exponential weighted moving average (EWMA) with time-adaptive alpha to track price volatility:

```
alpha(dt) = 1 - exp(-ln(2) × dt / half_life)
vol_estimate_t = alpha × vol_estimate_{t-1} + (1 - alpha) × r_t²
```

where:

- `vol_estimate_t` is the squared volatility estimate at time t
- `alpha(dt)` is the time-adaptive decay parameter that adjusts to the actual time interval
- `half_life` is the half-life parameter (in seconds) that controls the EWMA's memory
- `dt` is the actual time interval since the last update
- `r_t` is the log return normalized by the time interval: `(ln(price_t / price_{t-1}))² / dt`

This approach ensures consistent behavior even with unpredictable sample rates, as alpha adapts to the actual time between observations.

## Why Deterministic Tests?

We want to test the volatility estimation algorithm for different parameters, which requires random price paths, but we need the tests
to be reproducible.

## Files

- `generate_volatility_test_data.py` - Python script that generates all fixtures
- `*.json` - Individual test scenario fixtures. These are all generated and MUST NOT BE EDITED MANUALLY.
- `index.json` - Index of all available scenarios. This is also generated and MUST NOT BE EDITED MANUALLY.

## Time Units

All timestamps in the fixtures are in **milliseconds** to match blockchain block times. The Rust code measures time intervals in milliseconds and normalizes volatility accordingly (volatility per millisecond).

## Test Scenarios

### Single Regime Tests

Tests with a single volatility regime (20% annualized volatility, properly scaled to millisecond intervals):

- `single_regime_halflife_1s.json` - half-life = 1 second (fast adaptation)
- `single_regime_halflife_5s.json` - half-life = 5 seconds (medium adaptation)
- `single_regime_halflife_15s.json` - half-life = 15 seconds (slow adaptation)

Each contains 18,000 price points with 200ms intervals (1 hour of data).

### Multi-Phase Tests

Tests with changing volatility regimes (20% → 40% → 20%):

- `multi_phase_halflife_1s.json` - half-life = 1 second
- `multi_phase_halflife_5s.json` - half-life = 5 seconds
- `multi_phase_halflife_15s.json` - half-life = 15 seconds

Each contains 17,998 price points across three phases of 6,000 steps each.

### Variable Time Interval Tests

Tests with inconsistent time intervals (normally distributed) to simulate unpredictable block times. This is to test the time-adaptive alpha feature. All use a 5-second half-life and 18,000 samples (1 hour of data):

**200ms mean block time:**

- `variable_dt_200ms_low_variance.json` - std = 20ms (10% CV)
- `variable_dt_200ms_med_variance.json` - std = 50ms (25% CV)
- `variable_dt_200ms_high_variance.json` - std = 100ms (50% CV)

**600ms mean block time:**

- `variable_dt_600ms_low_variance.json` - std = 60ms (10% CV)
- `variable_dt_600ms_med_variance.json` - std = 150ms (25% CV)
- `variable_dt_600ms_high_variance.json` - std = 300ms (50% CV)

**1000ms mean block time:**

- `variable_dt_1000ms_low_variance.json` - std = 100ms (10% CV)
- `variable_dt_1000ms_med_variance.json` - std = 250ms (25% CV)
- `variable_dt_1000ms_high_variance.json` - std = 500ms (50% CV)

## Regenerating Fixtures

You MUST modify the Python script and regenerate the fixtures if you make any changes to the volatility estimation algorithm in the Rust code. If you want to add more test scenarios, you can edit the Python script and regenerate the fixtures.

```bash
# From the repository root
python3 dango/dex/tests/fixtures/generate_volatility_test_data.py
```

Or using uv:

```bash
uv run python dango/dex/tests/fixtures/generate_volatility_test_data.py
```

This will regenerate all fixture files.

## Running Tests

```bash
# Run all deterministic volatility tests
cargo test --package dango-dex --test volatility_deterministic

# Run a specific test
cargo test --package dango-dex --test volatility_deterministic test_single_regime_halflife_1s

# Run with output
cargo test --package dango-dex --test volatility_deterministic -- --nocapture
```

## Test Structure

1. **Fixture Module** (`tests/volatility_fixtures.rs`) - Rust structs for loading JSON fixtures
2. **Test File** (`tests/volatility_deterministic.rs`) - Integration tests that use fixtures
3. **Fixtures** (`tests/fixtures/*.json`) - Pre-generated test data

## Adding New Scenarios

To add a new test scenario:

1. Edit `generate_volatility_test_data.py` and add to `generate_test_scenarios()`
2. Regenerate fixtures (see above)
3. Add the scenario name to `TestScenario::load_all()` in `volatility_fixtures.rs`
4. Add a corresponding test function in `volatility_deterministic.rs`

## Precision

All values are stored as high-precision strings representing the raw u128 value with 24 decimal places, matching Rust's `Udec128` type.

Example: `"100000000000000004764729344"` represents 100.000000000000004764729344

This ensures exact matching between Python-generated expectations and Rust calculations.

## Verification

The Python implementation mirrors the Rust algorithm exactly:

- Same EWMA formula
- Same log return calculation
- Same time normalization
- Same precision (24 decimals)

This allows us to verify that the Rust implementation matches the mathematical specification.

## Test Results

All tests should pass with near-zero relative error (<0.000001). The tests verify:

1. **Correctness** - Estimates match Python reference implementation
2. **Convergence** - Estimates converge to target volatility over time
3. **Adaptation** - Estimates adapt when volatility changes
4. **Consistency** - Results are identical on every run
