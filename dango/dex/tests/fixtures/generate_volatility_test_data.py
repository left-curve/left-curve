#!/usr/bin/env python3
"""
Generate deterministic test data for volatility estimation tests.

This script:
1. Generates price paths using geometric Brownian motion with a fixed seed
2. Implements the same volatility estimation algorithm as the Rust code
3. Generates expected volatility estimates for multiple half-life values
4. Saves the data as JSON fixtures that Rust tests can load

The algorithm uses a time-adaptive EWMA where alpha adjusts to the actual
time interval between observations:
    alpha(dt) = 1 - exp(-ln(2) * dt / half_life)
    vol_estimate_t = (1 - alpha) * vol_estimate_{t-1} + alpha * r_t^2
"""

import json
import math
import numpy as np
from pathlib import Path
from typing import List, Tuple, Dict
from dataclasses import dataclass, asdict


@dataclass
class PricePoint:
    """A price observation at a specific timestamp."""

    timestamp: int  # milliseconds (all tests use milliseconds)
    price: str  # High-precision string representation (24 decimals)


@dataclass
class VolatilityEstimate:
    """A volatility estimate at a specific timestamp."""

    timestamp: int  # milliseconds
    estimate: str  # High-precision string representation (24 decimals)
    price: str  # The price at this timestamp


@dataclass
class TestScenario:
    """A complete test scenario with price path and expected estimates."""

    name: str
    description: str
    initial_price: str
    volatility: float  # The true volatility used to generate prices
    time_step_seconds: float  # Only for fixed-interval tests (e.g., 0.2 for 200ms)
    half_life_seconds: int  # Half-life parameter for EWMA
    price_path: List[PricePoint]
    expected_estimates: List[VolatilityEstimate]
    # Optional fields for variable interval tests
    mean_dt_ms: int = 0  # Mean time between steps in milliseconds
    std_dt_ms: int = 0  # Std dev of time between steps in milliseconds


class HighPrecisionNumber:
    """Handle numbers with 24 decimal places to match the dex::Price type."""

    PRECISION = 24
    SCALE = 10**PRECISION

    @classmethod
    def from_float(cls, value: float) -> str:
        """Convert float to high-precision string."""
        return str(int(value * cls.SCALE))

    @classmethod
    def to_float(cls, value: str) -> float:
        """Convert high-precision string to float."""
        return int(value) / cls.SCALE

    @classmethod
    def from_int(cls, value: int, decimals: int = 0) -> str:
        """Convert integer with given decimals to high-precision string."""
        return str(int(value * (10 ** (cls.PRECISION - decimals))))

    @classmethod
    def multiply(cls, a: str, b: str) -> str:
        """Multiply two high-precision numbers."""
        result = (int(a) * int(b)) // cls.SCALE
        return str(result)

    @classmethod
    def divide(cls, a: str, b: str) -> str:
        """Divide two high-precision numbers."""
        result = (int(a) * cls.SCALE) // int(b)
        return str(result)

    @classmethod
    def add(cls, a: str, b: str) -> str:
        """Add two high-precision numbers."""
        return str(int(a) + int(b))

    @classmethod
    def subtract(cls, a: str, b: str) -> str:
        """Subtract two high-precision numbers."""
        return str(int(a) - int(b))


def generate_price_path(
    initial_price: float,
    volatility: float,
    num_steps: int,
    time_step_seconds: int,
    seed: int = 42,
) -> List[Tuple[int, str]]:
    """
    Generate a price path following geometric Brownian motion.

    Args:
        initial_price: Starting price
        volatility: Volatility parameter (standard deviation of log returns per second)
        num_steps: Number of price points to generate
        time_step_seconds: Time between each step in seconds
        seed: Random seed for reproducibility

    Returns:
        List of (timestamp_ms, price_string) tuples with timestamps in milliseconds
    """
    np.random.seed(seed)

    prices = []
    current_price = initial_price
    current_time_ms = 0
    time_step_ms = int(time_step_seconds * 1000)  # Convert to integer milliseconds

    # Add initial price
    prices.append((current_time_ms, HighPrecisionNumber.from_float(current_price)))

    # Generate subsequent prices
    for _ in range(num_steps - 1):
        # Sample log return from normal distribution
        # Standard deviation scales with sqrt(time_step)
        log_return = np.random.normal(0, volatility * math.sqrt(time_step_seconds))

        # Apply the log return to get price ratio
        price_ratio = math.exp(log_return)

        # Clamp to prevent extreme values (matching Rust implementation)
        clamped_ratio = max(0.5, min(2.0, price_ratio))

        # Update price
        current_price *= clamped_ratio
        current_time_ms += time_step_ms

        prices.append((current_time_ms, HighPrecisionNumber.from_float(current_price)))

    return prices


def generate_price_path_variable_dt(
    initial_price: float,
    volatility: float,
    num_steps: int,
    mean_dt_ms: float,
    std_dt_ms: float,
    seed: int = 42,
) -> List[Tuple[int, str]]:
    """
    Generate a price path with variable time intervals between observations.

    Args:
        initial_price: Starting price
        volatility: Volatility parameter (standard deviation of log returns per second)
        num_steps: Number of price points to generate
        mean_dt_ms: Mean time between steps in milliseconds
        std_dt_ms: Standard deviation of time between steps in milliseconds
        seed: Random seed for reproducibility

    Returns:
        List of (timestamp, price_string) tuples
    """
    np.random.seed(seed)

    prices = []
    current_price = initial_price
    current_time_ms = 0

    # Add initial price
    prices.append((current_time_ms, HighPrecisionNumber.from_float(current_price)))

    # Generate subsequent prices with variable time steps
    for _ in range(num_steps - 1):
        # Sample time interval from normal distribution (in milliseconds)
        # Ensure it's positive and at least 10ms
        dt_ms = max(10, int(np.random.normal(mean_dt_ms, std_dt_ms)))
        dt_seconds = dt_ms / 1000.0

        # Sample log return from normal distribution
        # Standard deviation scales with sqrt(time_step)
        log_return = np.random.normal(0, volatility * math.sqrt(dt_seconds))

        # Apply the log return to get price ratio
        price_ratio = math.exp(log_return)

        # Clamp to prevent extreme values (matching Rust implementation)
        clamped_ratio = max(0.5, min(2.0, price_ratio))

        # Update price
        current_price *= clamped_ratio
        current_time_ms += dt_ms

        prices.append((current_time_ms, HighPrecisionNumber.from_float(current_price)))

    return prices


def compute_volatility_estimates(
    price_path: List[Tuple[int, str]], half_life_seconds: int
) -> List[Tuple[int, str, str]]:
    """
    Compute volatility estimates for a given price path.

    This implements the same algorithm as the Rust code with time-adaptive alpha:

    1. Measure the true interval: dt = now_timestamp - prev_timestamp
    2. Scale each squared return to "per-millisecond" units: v_i = (ln P_i - ln P_{i-1})^2 / dt_ms
    3. Use an EWMA whose α adapts to the interval:
       alpha(dt) = 1 - exp(-ln(2) * dt_ms / half_life_ms)
    4. Update: sigma2 = (1 - alpha) * sigma2 + alpha * v_i
       This gives weight alpha to the NEW observation and (1-alpha) to the OLD value,
       correctly implementing half-life semantics.

    Args:
        price_path: List of (timestamp_ms, price_string) tuples
                    Timestamps are in milliseconds
        half_life_seconds: Half-life parameter for EWMA in seconds

    Returns:
        List of (timestamp_ms, estimate_string, price_string) tuples
    """
    if len(price_path) < 2:
        return []

    estimates = []

    # First observation: estimate is zero
    prev_time, prev_price = price_path[0]
    prev_squared_vol = HighPrecisionNumber.from_float(0.0)
    estimates.append((prev_time, prev_squared_vol, prev_price))

    # Process remaining observations
    for curr_time, curr_price in price_path[1:]:
        # 1. Measure the true interval (already in milliseconds)
        dt_ms = curr_time - prev_time

        # 2. Compute log return: ln(price_t / price_{t-1})
        price_ratio = HighPrecisionNumber.to_float(
            curr_price
        ) / HighPrecisionNumber.to_float(prev_price)
        log_return = math.log(price_ratio)

        # Square the log return and normalize by time interval in milliseconds
        # This matches the Rust code which divides by time_diff_ms
        r_t_squared_norm = (log_return**2) / dt_ms

        # Convert to high-precision string
        r_t_squared_norm_str = HighPrecisionNumber.from_float(r_t_squared_norm)

        # 3. Calculate alpha that adapts to the time interval
        # alpha(dt) = 1 - exp(-ln(2) * dt_ms / half_life_ms)
        # Use the exact same ln(2) value as the Rust code (24 decimal places)
        ln_2 = 0.693147180559945309417232  # From NATURAL_LOG_OF_TWO constant in Rust
        half_life_ms = half_life_seconds * 1000
        alpha = 1.0 - math.exp(-ln_2 * dt_ms / half_life_ms)
        alpha_str = HighPrecisionNumber.from_float(alpha)
        one_minus_alpha_str = HighPrecisionNumber.from_float(1.0 - alpha)

        # 4. Update estimate: (1 - alpha) * vol_{t-1} + alpha * r_t^2
        # This gives weight alpha to NEW observation, (1-alpha) to OLD value
        # which correctly implements half-life semantics
        term1 = HighPrecisionNumber.multiply(one_minus_alpha_str, prev_squared_vol)
        term2 = HighPrecisionNumber.multiply(alpha_str, r_t_squared_norm_str)
        vol_estimate = HighPrecisionNumber.add(term1, term2)

        estimates.append((curr_time, vol_estimate, curr_price))

        # Update for next iteration
        prev_time = curr_time
        prev_price = curr_price
        prev_squared_vol = vol_estimate

    return estimates


def generate_test_scenarios() -> List[TestScenario]:
    """Generate multiple test scenarios with different parameters."""

    scenarios = []

    # Scenario 1: Single volatility regime with different half-life values
    # 18000 points at 200ms intervals = 1 hour of data
    initial_price = 100.0
    volatility1 = 0.2  # 20% volatility per second
    time_step = 0.2  # 0.2 seconds = 200ms per step (typical block time)
    num_samples = 18000

    price_path = generate_price_path(
        initial_price, volatility1, num_samples, time_step, seed=42
    )

    # Test with different half-life values:
    # - 1s: fast adaptation (similar to old lambda ≈ 0.59)
    # - 5s: medium adaptation (similar to old lambda ≈ 0.87)
    # - 15s: slow adaptation (similar to old lambda ≈ 0.95)
    for half_life, name_suffix in [(1, "1s"), (5, "5s"), (15, "15s")]:
        estimates = compute_volatility_estimates(price_path, half_life)

        scenarios.append(
            TestScenario(
                name=f"single_regime_halflife_{name_suffix}",
                description=f"Single volatility regime (20%) with half-life={half_life}s (18000 points = 1 hour)",
                initial_price=HighPrecisionNumber.from_float(initial_price),
                volatility=volatility1,
                time_step_seconds=time_step,
                half_life_seconds=half_life,
                price_path=[PricePoint(timestamp=t, price=p) for t, p in price_path],
                expected_estimates=[
                    VolatilityEstimate(timestamp=t, estimate=e, price=p)
                    for t, e, p in estimates
                ],
            )
        )

    # Scenario 2: Multi-phase volatility regimes
    # Phase 1: 20% volatility for 6000 steps (~20 minutes at 200ms blocks)
    phase1_prices = generate_price_path(initial_price, 0.2, 6000, time_step, seed=42)

    # Phase 2: 40% volatility for 6000 steps (continuing from phase 1 price)
    last_price_phase1 = HighPrecisionNumber.to_float(phase1_prices[-1][1])
    last_time_phase1 = phase1_prices[-1][0]
    phase2_prices = generate_price_path(
        last_price_phase1, 0.4, 6000, time_step, seed=43
    )
    # Adjust timestamps to continue from phase 1 (add time_step in milliseconds)
    time_step_ms = int(time_step * 1000)
    phase2_prices = [
        (t + last_time_phase1 + time_step_ms, p) for t, p in phase2_prices[1:]
    ]

    # Phase 3: Back to 20% volatility for 6000 steps
    last_price_phase2 = HighPrecisionNumber.to_float(phase2_prices[-1][1])
    last_time_phase2 = phase2_prices[-1][0]
    phase3_prices = generate_price_path(
        last_price_phase2, 0.2, 6000, time_step, seed=44
    )
    phase3_prices = [
        (t + last_time_phase2 + time_step_ms, p) for t, p in phase3_prices[1:]
    ]

    # Combine all phases
    multi_phase_path = phase1_prices + phase2_prices + phase3_prices

    for half_life, name_suffix in [(1, "1s"), (5, "5s"), (15, "15s")]:
        estimates = compute_volatility_estimates(multi_phase_path, half_life)

        scenarios.append(
            TestScenario(
                name=f"multi_phase_halflife_{name_suffix}",
                description=f"Three-phase volatility (20%->40%->20%) with half-life={half_life}s (18000 points = 1 hour)",
                initial_price=HighPrecisionNumber.from_float(initial_price),
                volatility=0.2,  # Initial volatility
                time_step_seconds=time_step,
                half_life_seconds=half_life,
                price_path=[
                    PricePoint(timestamp=t, price=p) for t, p in multi_phase_path
                ],
                expected_estimates=[
                    VolatilityEstimate(timestamp=t, estimate=e, price=p)
                    for t, e, p in estimates
                ],
            )
        )

    # Scenario 3: Variable time intervals (tests time-adaptive alpha)
    # This is the key benefit of the half-life approach!
    variable_dt_scenarios = generate_variable_dt_scenarios()
    scenarios.extend(variable_dt_scenarios)

    return scenarios


def generate_variable_dt_scenarios() -> List[TestScenario]:
    """Generate test scenarios with variable time intervals."""

    scenarios = []
    initial_price = 100.0
    volatility = 0.2  # 20% volatility per second
    num_samples = 18000  # 1 hour of data at various average block times
    half_life = 5  # 5 second half-life for all variable dt tests

    # Test different mean intervals with different variances
    test_configs = [
        # (mean_dt_ms, std_dt_ms, description_suffix)
        # 200ms mean
        (200, 20, "200ms_low_variance"),  # 10% CV
        (200, 50, "200ms_med_variance"),  # 25% CV
        (200, 100, "200ms_high_variance"),  # 50% CV
        # 600ms mean
        (600, 60, "600ms_low_variance"),  # 10% CV
        (600, 150, "600ms_med_variance"),  # 25% CV
        (600, 300, "600ms_high_variance"),  # 50% CV
        # 1000ms mean
        (1000, 100, "1000ms_low_variance"),  # 10% CV
        (1000, 250, "1000ms_med_variance"),  # 25% CV
        (1000, 500, "1000ms_high_variance"),  # 50% CV
    ]

    for mean_dt_ms, std_dt_ms, desc_suffix in test_configs:
        # Use different seeds for each scenario
        seed = 100 + len(scenarios)

        price_path = generate_price_path_variable_dt(
            initial_price, volatility, num_samples, mean_dt_ms, std_dt_ms, seed=seed
        )

        estimates = compute_volatility_estimates(price_path, half_life)

        scenarios.append(
            TestScenario(
                name=f"variable_dt_{desc_suffix}",
                description=f"Variable intervals (mean={mean_dt_ms}ms, std={std_dt_ms}ms, half-life=5s)",
                initial_price=HighPrecisionNumber.from_float(initial_price),
                volatility=volatility,
                time_step_seconds=0,  # Not applicable for variable dt
                half_life_seconds=half_life,
                mean_dt_ms=mean_dt_ms,
                std_dt_ms=std_dt_ms,
                price_path=[PricePoint(timestamp=t, price=p) for t, p in price_path],
                expected_estimates=[
                    VolatilityEstimate(timestamp=t, estimate=e, price=p)
                    for t, e, p in estimates
                ],
            )
        )

    return scenarios


def save_scenarios(scenarios: List[TestScenario], output_dir: Path):
    """Save test scenarios as JSON files."""
    output_dir.mkdir(parents=True, exist_ok=True)

    # Save each scenario to a separate file
    for scenario in scenarios:
        filename = output_dir / f"{scenario.name}.json"
        with open(filename, "w") as f:
            json.dump(asdict(scenario), f, indent=2)
        print(f"Saved {filename}")

    # Also save an index file listing all scenarios
    index = {
        "scenarios": [scenario.name for scenario in scenarios],
        "description": "Deterministic test data for volatility estimation",
    }
    with open(output_dir / "index.json", "w") as f:
        json.dump(index, f, indent=2)
    print(f"Saved index file")


def main():
    """Generate and save all test scenarios."""
    print("Generating volatility test scenarios...")
    scenarios = generate_test_scenarios()

    print(f"\nGenerated {len(scenarios)} scenarios:")
    for scenario in scenarios:
        print(
            f"  - {scenario.name}: {len(scenario.price_path)} prices, "
            f"{len(scenario.expected_estimates)} estimates"
        )

    # Save to fixtures directory
    output_dir = Path(__file__).parent
    save_scenarios(scenarios, output_dir)

    print(f"\nTest data saved to {output_dir}")
    print("Done!")


if __name__ == "__main__":
    main()
