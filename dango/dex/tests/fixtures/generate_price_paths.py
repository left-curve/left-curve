#!/usr/bin/env python3
"""
Generate price paths using geometric Brownian motion.

This module provides functions to generate deterministic price paths
that can be used for testing various features including volatility estimation.
"""

import json
import math
import numpy as np
from pathlib import Path
from typing import List, Tuple
from dataclasses import dataclass, asdict


@dataclass
class PricePoint:
    """A price observation at a specific timestamp."""

    timestamp: int  # milliseconds (all tests use milliseconds)
    price: str  # High-precision string representation (24 decimals)


@dataclass
class PricePath:
    """A price path with metadata."""

    name: str
    description: str
    initial_price: str
    volatility: float  # The true volatility used to generate prices
    time_step_seconds: (
        float  # Only for fixed-interval tests (e.g., 0.2 for 200ms), 0 for variable
    )
    prices: List[PricePoint]  # Array of price points
    mean_dt_ms: int = 0  # Mean time between steps in milliseconds
    std_dt_ms: int = 0  # Std dev of time between steps in milliseconds (0 for fixed dt)
    seed: int = 42  # Random seed used for generation


class HighPrecisionNumber:
    """Handle numbers with 24 decimal places to match the dex::Price type."""

    PRECISION = 24
    SCALE = 10**PRECISION
    # Maximum value for u128: 2^128 - 1 = 340282366920938463463374607431768211455
    MAX_U128 = 2**128 - 1
    # Maximum representable price: (2^128 - 1) / 10^24 ≈ 340.28 trillion
    MAX_PRICE = MAX_U128 / SCALE

    @classmethod
    def from_float(cls, value: float) -> str:
        """Convert float to high-precision string."""
        if value > cls.MAX_PRICE:
            raise ValueError(
                f"Price {value} exceeds maximum representable price {cls.MAX_PRICE} "
                f"(u128::MAX / 10^{cls.PRECISION})"
            )
        if value < 0:
            raise ValueError(f"Price {value} cannot be negative")
        scaled = int(value * cls.SCALE)
        if scaled > cls.MAX_U128:
            raise ValueError(
                f"Scaled price {scaled} exceeds u128::MAX {cls.MAX_U128}"
            )
        return str(scaled)

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
        std_dt_ms: Standard deviation of time between steps in milliseconds.
                   Pass 0 for fixed intervals (will always use mean_dt_ms).
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
    for step in range(num_steps - 1):
        # Sample time interval from normal distribution (in milliseconds)
        # Ensure it's positive and at least 10ms, and cap at 3 standard deviations
        # to prevent extreme outliers that cause excessive price drift
        dt_ms_raw = np.random.normal(mean_dt_ms, std_dt_ms)
        dt_ms = max(10, min(int(dt_ms_raw), int(mean_dt_ms + 3 * std_dt_ms)))
        dt_seconds = dt_ms / 1000.0

        # Sample log return from normal distribution
        # Standard deviation scales with sqrt(time_step)
        log_return = np.random.normal(0, volatility * math.sqrt(dt_seconds))

        # Apply the log return to get price ratio
        price_ratio = math.exp(log_return)

        # Update price
        current_price *= price_ratio
        
        # Clamp price to maximum representable value to prevent u128 overflow
        if current_price > HighPrecisionNumber.MAX_PRICE:
            current_price = HighPrecisionNumber.MAX_PRICE
            print(
                f"Warning: Price at step {step + 1} exceeded maximum ({HighPrecisionNumber.MAX_PRICE:.2f}), "
                f"clamped to maximum"
            )
        
        current_time_ms += dt_ms

        prices.append((current_time_ms, HighPrecisionNumber.from_float(current_price)))

    return prices


def generate_price_paths() -> List[PricePath]:
    """Generate all price paths with their metadata."""

    price_paths = []

    # Price Path 1: Single volatility regime
    # 18000 points at 200ms intervals = 1 hour of data
    initial_price = 100.0
    # Volatility: 0.2% per second (0.002) = ~113% annualized, which is high but realistic for crypto
    # This is much more reasonable than 20% per second (which would be 112,352% annualized!)
    volatility1 = 0.002  # 0.2% volatility per second
    time_step = 0.2  # 0.2 seconds = 200ms per step (typical block time)
    num_samples = 18000
    time_step_ms = int(time_step * 1000)  # Convert to milliseconds

    # Fixed dt scenario: std_dt_ms = 0
    price_path_tuples = generate_price_path_variable_dt(
        initial_price, volatility1, num_samples, time_step_ms, 0, seed=42
    )

    price_paths.append(
        PricePath(
            name="single_regime_200ms",
            description="Single volatility regime (20%) with fixed 200ms intervals (18000 points = 1 hour)",
            initial_price=HighPrecisionNumber.from_float(initial_price),
            volatility=volatility1,
            time_step_seconds=time_step,
            mean_dt_ms=time_step_ms,
            std_dt_ms=0,
            seed=42,
            prices=[PricePoint(timestamp=t, price=p) for t, p in price_path_tuples],
        )
    )

    # Price Path 2: Multi-phase volatility regimes
    # Phase 1: 0.2% volatility per second for 6000 steps (~20 minutes at 200ms blocks)
    # Fixed dt scenario: std_dt_ms = 0
    phase1_prices = generate_price_path_variable_dt(
        initial_price, 0.002, 6000, time_step_ms, 0, seed=42
    )

    # Phase 2: 0.4% volatility per second for 6000 steps (continuing from phase 1 price)
    last_price_phase1 = HighPrecisionNumber.to_float(phase1_prices[-1][1])
    last_time_phase1 = phase1_prices[-1][0]
    phase2_prices = generate_price_path_variable_dt(
        last_price_phase1, 0.004, 6000, time_step_ms, 0, seed=43
    )
    # Adjust timestamps to continue from phase 1 (add time_step in milliseconds)
    phase2_prices = [
        (t + last_time_phase1 + time_step_ms, p) for t, p in phase2_prices[1:]
    ]

    # Phase 3: Back to 0.2% volatility per second for 6000 steps
    last_price_phase2 = HighPrecisionNumber.to_float(phase2_prices[-1][1])
    last_time_phase2 = phase2_prices[-1][0]
    phase3_prices = generate_price_path_variable_dt(
        last_price_phase2, 0.002, 6000, time_step_ms, 0, seed=44
    )
    phase3_prices = [
        (t + last_time_phase2 + time_step_ms, p) for t, p in phase3_prices[1:]
    ]

    # Combine all phases
    multi_phase_path = phase1_prices + phase2_prices + phase3_prices

    price_paths.append(
        PricePath(
            name="multi_phase_200ms",
            description="Three-phase volatility (0.2%->0.4%->0.2% per second) with fixed 200ms intervals (18000 points = 1 hour)",
            initial_price=HighPrecisionNumber.from_float(initial_price),
            volatility=0.002,  # Initial volatility (0.2% per second)
            time_step_seconds=time_step,
            mean_dt_ms=time_step_ms,
            std_dt_ms=0,
            seed=42,  # Initial seed
            prices=[PricePoint(timestamp=t, price=p) for t, p in multi_phase_path],
        )
    )

    # Price Path 3+: Variable time intervals
    # Variable dt scenarios: std_dt_ms > 0
    initial_price_var = 100.0
    # Volatility: 0.2% per second (0.002) = ~113% annualized, which is high but realistic for crypto
    volatility = 0.002  # 0.2% volatility per second
    num_samples_var = 18000  # 1 hour of data at various average block times

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
        seed = 100 + len([p for p in price_paths if p.name.startswith("variable_dt_")])

        price_path_tuples = generate_price_path_variable_dt(
            initial_price_var,
            volatility,
            num_samples_var,
            mean_dt_ms,
            std_dt_ms,
            seed=seed,
        )

        price_paths.append(
            PricePath(
                name=f"variable_dt_{desc_suffix}",
                description=f"Variable intervals (mean={mean_dt_ms}ms, std={std_dt_ms}ms)",
                initial_price=HighPrecisionNumber.from_float(initial_price_var),
                volatility=volatility,
                time_step_seconds=0,  # Not applicable for variable dt
                mean_dt_ms=mean_dt_ms,
                std_dt_ms=std_dt_ms,
                seed=seed,
                prices=[PricePoint(timestamp=t, price=p) for t, p in price_path_tuples],
            )
        )

    return price_paths


def save_price_paths(price_paths: List[PricePath], output_dir: Path):
    """Save price paths as JSON files in a price_paths subdirectory."""
    price_paths_dir = output_dir / "price_paths"
    price_paths_dir.mkdir(parents=True, exist_ok=True)

    for price_path in price_paths:
        filename = price_paths_dir / f"{price_path.name}.json"
        with open(filename, "w") as f:
            json.dump(asdict(price_path), f, indent=2)
        print(f"Saved price path: {filename}")


def main():
    """Generate and save all price paths."""
    print("Generating price paths...")
    price_paths = generate_price_paths()

    print(f"\nGenerated {len(price_paths)} price paths:")
    for price_path in price_paths:
        print(f"  - {price_path.name}: {len(price_path.prices)} prices")

    # Save to fixtures directory
    output_dir = Path(__file__).parent
    save_price_paths(price_paths, output_dir)

    print(f"\nPrice path files saved to {output_dir / 'price_paths'}")
    print("Done!")


if __name__ == "__main__":
    main()

