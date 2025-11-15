# Deterministic Volatility Estimation Test Fixtures

This directory contains deterministic test fixtures for the volatility estimation algorithm used in the DEX.

## Overview

The volatility estimator uses an exponential weighted moving average (EWMA) to track price volatility:

```
vol_estimate_t = λ × vol_estimate_{t-1} + (1 - λ) × r_t²
```

where:
- `vol_estimate_t` is the squared volatility estimate at time t
- `λ` is the decay parameter (smoothing factor)
- `r_t` is the log return normalized by the time interval: `(ln(price_t / price_{t-1}))² / Δt`

## Why Deterministic Tests?

Previously, tests used random price paths which led to:
1. **Non-deterministic results** - tests could pass or fail randomly
2. **Hard to debug** - couldn't reproduce specific failures
3. **Unclear expectations** - no reference implementation to verify correctness

This framework solves these issues by:
1. **Generating deterministic price paths** using a fixed random seed
2. **Computing expected results** using a Python reference implementation
3. **Storing fixtures** as JSON files that Rust tests load

## Files

- `generate_volatility_test_data.py` - Python script that generates all fixtures
- `*.json` - Individual test scenario fixtures
- `index.json` - Index of all available scenarios

## Test Scenarios

### Single Regime Tests
Tests with a single volatility regime (20% volatility):
- `single_regime_lambda_90.json` - λ = 0.9 (faster convergence)
- `single_regime_lambda_95.json` - λ = 0.95 (medium convergence)  
- `single_regime_lambda_99.json` - λ = 0.99 (slower convergence)

Each contains 150 price points with 1 second intervals.

### Multi-Phase Tests
Tests with changing volatility regimes (20% → 40% → 20%):
- `multi_phase_lambda_90.json` - λ = 0.9
- `multi_phase_lambda_95.json` - λ = 0.95
- `multi_phase_lambda_99.json` - λ = 0.99

Each contains 448 price points across three phases of 150 steps each.

## Regenerating Fixtures

If you modify the volatility estimation algorithm or want to add new test scenarios:

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
cargo test --package dango-dex --test volatility_deterministic test_single_regime_lambda_90

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

