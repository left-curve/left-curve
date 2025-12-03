#!/usr/bin/env python3
"""
Generate volatility estimation test data from existing price paths.

This script:
1. Reads price paths from the price_paths folder
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
from pathlib import Path
from typing import List
from dataclasses import dataclass, asdict

from generate_price_paths import (
    PricePoint,
    PricePath,
    HighPrecisionNumber,
)


@dataclass
class VolatilityEstimateFile:
    """Volatility estimates for a price path."""

    name: str
    description: str
    price_path_file: str  # Name of the price path file this estimates
    half_life_seconds: int  # Half-life parameter for EWMA
    estimates: List[
        str
    ]  # Array of volatility estimates (high-precision strings, one per price point)


def compute_volatility_estimates(
    price_path: List[PricePoint], half_life_seconds: int
) -> List[str]:
    """
    Compute volatility estimates for a given price path.

    This implements the same algorithm as the Rust code with time-adaptive alpha:

    1. Measure the true interval: dt = now_timestamp - prev_timestamp
    2. Scale each squared return to "per-second" units: v_i = (ln P_i - ln P_{i-1})^2 / dt_seconds
    3. Use an EWMA whose α adapts to the interval:
       alpha(dt) = 1 - exp(-ln(2) * dt_ms / half_life_ms)
    4. Update: sigma2 = (1 - alpha) * sigma2 + alpha * v_i
       This gives weight alpha to the NEW observation and (1-alpha) to the OLD value,
       correctly implementing half-life semantics.

    Args:
        price_path: List of PricePoint objects
                    Timestamps are in milliseconds
        half_life_seconds: Half-life parameter for EWMA in seconds

    Returns:
        List of volatility estimate strings (high-precision, one per price point)
        The estimates correspond exactly to the price_path indices (estimate[i] for price_path[i])
    """
    if len(price_path) < 1:
        return []

    estimates = []

    # First observation: estimate is zero
    prev_point = price_path[0]
    prev_squared_vol = HighPrecisionNumber.from_float(0.0)
    estimates.append(prev_squared_vol)

    # Process remaining observations
    for curr_point in price_path[1:]:
        # 1. Measure the true interval (already in milliseconds)
        dt_ms = curr_point.timestamp - prev_point.timestamp

        # 2. Compute log return: ln(price_t / price_{t-1})
        price_ratio = HighPrecisionNumber.to_float(
            curr_point.price
        ) / HighPrecisionNumber.to_float(prev_point.price)
        log_return = math.log(price_ratio)

        # Normalize by time interval in seconds
        dt_seconds = dt_ms / 1000.0
        r_t_squared_norm = (log_return**2) / dt_seconds

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

        estimates.append(vol_estimate)

        # Update for next iteration
        prev_point = curr_point
        prev_squared_vol = vol_estimate

    return estimates


def load_price_paths(price_paths_dir: Path) -> List[PricePath]:
    """Load all price paths from JSON files in the price_paths directory."""
    price_paths = []

    if not price_paths_dir.exists():
        raise FileNotFoundError(
            f"Price paths directory not found: {price_paths_dir}\n"
            f"Please run generate_price_paths.py first to generate the price paths."
        )

    # Find all JSON files in the price_paths directory
    for json_file in sorted(price_paths_dir.glob("*.json")):
        with open(json_file, "r") as f:
            data = json.load(f)

        # Convert the JSON data back to PricePath object
        price_points = [
            PricePoint(timestamp=pp["timestamp"], price=pp["price"])
            for pp in data["prices"]
        ]

        price_path = PricePath(
            name=data["name"],
            description=data["description"],
            initial_price=data["initial_price"],
            volatility=data["volatility"],
            time_step_seconds=data["time_step_seconds"],
            prices=price_points,
            mean_dt_ms=data.get("mean_dt_ms", 0),
            std_dt_ms=data.get("std_dt_ms", 0),
            seed=data.get("seed", 42),
        )
        price_paths.append(price_path)
        print(f"Loaded price path: {json_file.name}")

    return price_paths


def generate_volatility_estimates(
    price_paths: List[PricePath],
    half_lives: List[int],
) -> List[VolatilityEstimateFile]:
    """Generate volatility estimates for all price paths with different half-lives."""

    estimate_files = []

    for price_path in price_paths:
        # Generate estimates for each half-life value
        for half_life in half_lives:
            estimates = compute_volatility_estimates(price_path.prices, half_life)

            estimate_files.append(
                VolatilityEstimateFile(
                    name=f"volatility_estimates_{price_path.name}_halflife_{half_life}s",
                    description=f"Volatility estimates for {price_path.name} with half-life={half_life}s",
                    price_path_file=f"{price_path.name}.json",
                    half_life_seconds=half_life,
                    estimates=estimates,
                )
            )

    return estimate_files


def save_volatility_estimates(
    estimate_files: List[VolatilityEstimateFile], output_dir: Path
):
    """Save volatility estimate files as JSON files in a volatility_estimates subdirectory."""
    volatility_estimates_dir = output_dir / "volatility_estimates"
    volatility_estimates_dir.mkdir(parents=True, exist_ok=True)

    for estimate_file in estimate_files:
        filename = volatility_estimates_dir / f"{estimate_file.name}.json"
        # Convert to dict manually since estimates is now List[str] not List[VolatilityEstimate]
        data = {
            "name": estimate_file.name,
            "description": estimate_file.description,
            "price_path_file": estimate_file.price_path_file,
            "half_life_seconds": estimate_file.half_life_seconds,
            "estimates": estimate_file.estimates,
        }
        with open(filename, "w") as f:
            json.dump(data, f, indent=2)
        print(f"Saved volatility estimates: {filename}")


def main():
    """Load price paths and generate volatility estimates for all half-life configurations."""
    fixtures_dir = Path(__file__).parent
    price_paths_dir = fixtures_dir / "price_paths"

    print("Loading price paths...")
    price_paths = load_price_paths(price_paths_dir)

    print(f"\nLoaded {len(price_paths)} price paths:")
    for price_path in price_paths:
        print(f"  - {price_path.name}: {len(price_path.prices)} prices")

    # Half-life values to test for all price paths
    half_lives = [1, 5, 15]  # 1s, 5s, 15s

    print(f"\nGenerating volatility estimates for half-lives: {half_lives}...")
    estimate_files = generate_volatility_estimates(price_paths, half_lives)

    print(f"\nGenerated {len(estimate_files)} volatility estimate files:")
    for estimate_file in estimate_files:
        print(
            f"  - {estimate_file.name}: {len(estimate_file.estimates)} estimates "
            f"(for {estimate_file.price_path_file}, half-life={estimate_file.half_life_seconds}s)"
        )

    # Save to fixtures directory
    save_volatility_estimates(estimate_files, fixtures_dir)

    print(f"\nVolatility estimate files saved to {fixtures_dir}")
    print("Done!")


if __name__ == "__main__":
    main()
