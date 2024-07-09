# Gas

Some thoughts on how we define gas cost in Grug.

The Wasmer runtime provides a [`Metering`](https://docs.rs/wasmer-middlewares/latest/wasmer_middlewares/metering/struct.Metering.html) middleware that measures how many "points" a Wasm function call consumes.

The question is how to associate Wasmer points to the chain's gas units.

## CosmWasm's approach

As documented [here](https://github.com/CosmWasm/cosmwasm/blob/main/docs/GAS.md), CosmWasm's approach is as follows:

1. Perform a benchmark to measure how many "points" Wasmer can execute per second. Then, set a target amount of gas per second (they use 10^12 gas per second). Between these two numbers, CosmWasm decides that 1 Wasmer point is to equal 170 gas units.

2. Perform another benchmark to measure how much time it takes for the host to execute each host function (e.g. `addr_validate` or `secp256k1_verify`). Based on this, assign a proper gas cost for each host function.

3. Divide CosmWasm gas by a constant factor of 100 to arrive at Cosmos SDK gas.

## Our approach

For us, defining gas cost is easier, because we don't have a Cosmos SDK to deal with.

1. We skip step 1, and simply set 1 Wasmer point = 1 Grug gas unit.

2. We perform the same benchmarks to set proper gas costs for host functions.

3. We skip this step as well.

## Benchmark results

All following benchmarks are performed on a MacBook Pro with the M2 Pro CPU.

### Wasmer points per second

This corresponds to the step 1 above. This benchmark is irrelevant for our decision making (as we simply set 1 Wasmer point = 1 Grug gas unit), but we still perform it for good measure.

This benchmark utilizes the [contracts/testers/benchmarker](../../contracts/testers/benchmarker/) contract; relevant code can be found in [crates/vm/wasm/benches](../../crates/vm/wasm/benches/).

| Iterations | Points      | Time (ms) |
| ---------- | ----------- | --------- |
| 200,000    | 159,807,119 | 15.661    |
| 400,000    | 319,607,119 | 31.663    |
| 600,000    | 479,407,119 | 47.542    |
| 800,000    | 639,207,119 | 62.783    |
| 1,000,000  | 799,007,154 | 78.803    |

Extrapolating to 1 second, we arrive at that `WasmVm` executes 10,150,897,988 points per second.

If we were to target 10^12 gas units per second as CosmWasm does (we don't), this would mean 10^12 / 10,150,897,988 = 98 gas units per Wasmer point.

This is roughly in the same ballpark as CosmWasm's result (170 gas units per Wasmer point). The results are of course not directly comparable because they were done using different CPUs, but the numbers being within one degree of magnitude suggests the two VMs are similar in performance.
