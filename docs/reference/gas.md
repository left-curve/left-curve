# Gas

The Wasmer runtime provides a [`Metering`](https://docs.rs/wasmer-middlewares/latest/wasmer_middlewares/metering/struct.Metering.html) that measures how many "points" a Wasm function call consumes.

To associate this gas, we need to measure two things:

1. How many "points" can Wasmer execute per second. This way, given a target of a specific amount of gas per second (e.g. 10^12 gas per second), we can compute the ratio between Wasmer points and gas units.
2. How many host functions calls (e.g. `secp256k1_verify` and `sha2_256`) can the host execute per second. This way, we can assign a proper gas cost for each host function.

All following benchmarks are performed on a MacBook Pro with the M2 Pro CPU.

Utilizing the `grug-tester-benchmarker` contract, measuring Wasmer points per second:

| Iterations | Points      | Time (ms) |
| ---------- | ----------- | --------- |
| 200,000    | 159,807,119 | 15.661    |
| 400,000    | 319,607,119 | 31.663    |
| 600,000    | 479,407,119 | 47.542    |
| 800,000    | 639,207,119 | 62.783    |
| 1,000,000  | 799,007,154 | 78.803    |

Extrapolating to 1 second, we arrive at that `WasmVm` executes 10,150,897,988 points per second.

Targeting 10^12 gas units per second, this gives 10^12 / 10,150,897,988 = 99 gas units per Wasmer point. We round it up to 100 u
gas units per Wasmer point for simplicity.
