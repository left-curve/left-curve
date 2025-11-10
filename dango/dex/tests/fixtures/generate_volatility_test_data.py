#!/usr/bin/env python3
"""
Generate deterministic test data for volatility estimation tests.

This script:
1. Generates price paths using geometric Brownian motion with a fixed seed
2. Implements the same volatility estimation algorithm as the Rust code
3. Generates expected volatility estimates for multiple lambda values
4. Saves the data as JSON fixtures that Rust tests can load
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

    timestamp: int  # seconds
    price: str  # High-precision string representation (24 decimals)


@dataclass
class VolatilityEstimate:
    """A volatility estimate at a specific timestamp."""

    timestamp: int  # seconds
    estimate: str  # High-precision string representation (24 decimals)
    price: str  # The price at this timestamp


@dataclass
class TestScenario:
    """A complete test scenario with price path and expected estimates."""

    name: str
    description: str
    initial_price: str
    volatility: float  # The true volatility used to generate prices
    time_step_seconds: int
    lambda_value: str  # As high-precision string
    price_path: List[PricePoint]
    expected_estimates: List[VolatilityEstimate]


class HighPrecisionNumber:
    """Handle numbers with 24 decimal places to match Rust's Udec128."""

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
        List of (timestamp, price_string) tuples
    """
    np.random.seed(seed)

    prices = []
    current_price = initial_price
    current_time = 0

    # Add initial price
    prices.append((current_time, HighPrecisionNumber.from_float(current_price)))

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
        current_time += time_step_seconds

        prices.append((current_time, HighPrecisionNumber.from_float(current_price)))

    return prices


def compute_volatility_estimates(
    price_path: List[Tuple[int, str]], lambda_value: float
) -> List[Tuple[int, str, str]]:
    """
    Compute volatility estimates for a given price path.

    This implements the same algorithm as the Rust code:
    vol_estimate_t = lambda * vol_estimate_{t-1} + (1 - lambda) * r_t^2

    where r_t is the log return normalized by time interval.

    Args:
        price_path: List of (timestamp, price_string) tuples
        lambda_value: Decay parameter for exponential moving average

    Returns:
        List of (timestamp, estimate_string, price_string) tuples
    """
    if len(price_path) < 2:
        return []

    estimates = []
    lambda_str = HighPrecisionNumber.from_float(lambda_value)
    one_minus_lambda_str = HighPrecisionNumber.from_float(1.0 - lambda_value)

    # First observation: estimate is zero
    prev_time, prev_price = price_path[0]
    prev_squared_vol = HighPrecisionNumber.from_float(0.0)
    estimates.append((prev_time, prev_squared_vol, prev_price))

    # Process remaining observations
    for curr_time, curr_price in price_path[1:]:
        # Compute log return: ln(price_t / price_{t-1})
        price_ratio = HighPrecisionNumber.to_float(
            curr_price
        ) / HighPrecisionNumber.to_float(prev_price)
        log_return = math.log(price_ratio)

        # Square the log return
        r_t_squared = log_return**2

        # Normalize by time interval
        time_delta = curr_time - prev_time
        r_t_squared_norm = r_t_squared / time_delta

        # Convert to high-precision string
        r_t_squared_norm_str = HighPrecisionNumber.from_float(r_t_squared_norm)

        # Update estimate: lambda * vol_{t-1} + (1 - lambda) * r_t^2
        term1 = HighPrecisionNumber.multiply(lambda_str, prev_squared_vol)
        term2 = HighPrecisionNumber.multiply(one_minus_lambda_str, r_t_squared_norm_str)
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

    # Scenario 1: Single volatility regime with lambda=0.9
    initial_price = 100.0
    volatility1 = 0.2  # 20% volatility per second
    time_step = 1  # 1 second per step
    num_samples = 150

    price_path = generate_price_path(
        initial_price, volatility1, num_samples, time_step, seed=42
    )

    for lambda_val, lambda_name in [(0.9, "90"), (0.95, "95"), (0.99, "99")]:
        estimates = compute_volatility_estimates(price_path, lambda_val)

        scenarios.append(
            TestScenario(
                name=f"single_regime_lambda_{lambda_name}",
                description=f"Single volatility regime (20%) with lambda={lambda_val}",
                initial_price=HighPrecisionNumber.from_float(initial_price),
                volatility=volatility1,
                time_step_seconds=time_step,
                lambda_value=HighPrecisionNumber.from_float(lambda_val),
                price_path=[PricePoint(timestamp=t, price=p) for t, p in price_path],
                expected_estimates=[
                    VolatilityEstimate(timestamp=t, estimate=e, price=p)
                    for t, e, p in estimates
                ],
            )
        )

    # Scenario 2: Multi-phase volatility regimes
    # Phase 1: 20% volatility for 150 steps
    phase1_prices = generate_price_path(initial_price, 0.2, 150, time_step, seed=42)

    # Phase 2: 40% volatility for 150 steps (continuing from phase 1 price)
    last_price_phase1 = HighPrecisionNumber.to_float(phase1_prices[-1][1])
    last_time_phase1 = phase1_prices[-1][0]
    phase2_prices = generate_price_path(last_price_phase1, 0.4, 150, time_step, seed=43)
    # Adjust timestamps to continue from phase 1
    phase2_prices = [
        (t + last_time_phase1 + time_step, p) for t, p in phase2_prices[1:]
    ]

    # Phase 3: Back to 20% volatility for 150 steps
    last_price_phase2 = HighPrecisionNumber.to_float(phase2_prices[-1][1])
    last_time_phase2 = phase2_prices[-1][0]
    phase3_prices = generate_price_path(last_price_phase2, 0.2, 150, time_step, seed=44)
    phase3_prices = [
        (t + last_time_phase2 + time_step, p) for t, p in phase3_prices[1:]
    ]

    # Combine all phases
    multi_phase_path = phase1_prices + phase2_prices + phase3_prices

    for lambda_val, lambda_name in [(0.9, "90"), (0.95, "95"), (0.99, "99")]:
        estimates = compute_volatility_estimates(multi_phase_path, lambda_val)

        scenarios.append(
            TestScenario(
                name=f"multi_phase_lambda_{lambda_name}",
                description=f"Three-phase volatility (20%->40%->20%) with lambda={lambda_val}",
                initial_price=HighPrecisionNumber.from_float(initial_price),
                volatility=0.2,  # Initial volatility
                time_step_seconds=time_step,
                lambda_value=HighPrecisionNumber.from_float(lambda_val),
                price_path=[
                    PricePoint(timestamp=t, price=p) for t, p in multi_phase_path
                ],
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
