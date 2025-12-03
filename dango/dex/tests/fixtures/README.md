# Price statistics test fixtures

This directory contains fixtures for price statistics tests. Currently, it contains price paths and volatility estimates for the volatility estimator test.

### Price paths

All the price paths are located in the `price_paths` subdirectory.
They are generated using the `generate_price_paths.py` script. They follow a geometric Brownian motion with a fixed seed. Each file contains a price path with a fixed number of price points and metadata specifying the parameters used to generate the price path.

### Volatility estimates

Volatility estimates are located in the `volatility_estimates` subdirectory.
They are generated using the `generate_volatility_test_data.py` script. They use the price paths in the `price_paths` subdirectory and generate volatility estimate timeseries for different half-life values. Each file contains a list of volatility estimates for a given price path and half-life value.
The volatility estimates are generated using the same algorithm as the Rust code, i.e. a time-adaptive EWMA of log returns, where alpha is adapted to the time interval between observations.

## Regenerating the fixtures

To regenerate the price paths run the `generate_price_paths.py` script. After the price paths are re-generated, the corresponding volatility estimates must be so as well, by running the `generate_volatility_test_data.py` script.

```bash
python generate_price_paths.py
python generate_volatility_test_data.py
```

The volatility estimates can be regenerated without regenerating the price paths.
