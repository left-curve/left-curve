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

Benchmarks were performed on a MacBook Pro with the M2 Pro CPU.

Relevant code can be found in [crates/vm/wasm/benches](../../crates/vm/wasm/benches/) and [crates/crypto/benches](../../crates/crypto/benches/).

### Wasmer points per second

This corresponds to the step 1 above. This benchmark is irrelevant for our decision making (as we simply set 1 Wasmer point = 1 Grug gas unit), but we still perform it for good measure.

| Iterations | Points      | Time (ms) |
| ---------- | ----------- | --------- |
| 200,000    | 159,807,119 | 15.661    |
| 400,000    | 319,607,119 | 31.663    |
| 600,000    | 479,407,119 | 47.542    |
| 800,000    | 639,207,119 | 62.783    |
| 1,000,000  | 799,007,154 | 78.803    |

Extrapolating to 1 second, we arrive at that `WasmVm` executes 10,026,065,176 points per second. Let's round this to 10^10 points per second, for simplicity.

If we were to target 10^12 gas units per second as CosmWasm does (we don't), this would mean 10^12 / 10^10 = 100 gas units per Wasmer point.

This is roughly in the same ballpark as CosmWasm's result (170 gas units per Wasmer point). The results are of course not directly comparable because they were done using different CPUs, but the numbers being within one degree of magnitude suggests the two VMs are similar in performance.

As said before, we set 1 Wasmer point = 1 gas unit, so we're doing 10^10 gas per second.

### Single signature verification

Time for verifying one signature:

| Verifier                   | Time (ms) | Gas Per Verify |
| -------------------------- | --------- | -------------- |
| `secp256r1_verify`         | 0.188     | 1,880,000      |
| `secp256k1_verify`         | 0.077     | 770,000        |
| `secp256k1_pubkey_recover` | 0.158     | 1,580,000      |
| `ed25519_verify`           | 0.041     | 410,000        |

We have established that 1 second corresponds to 10^10 gas units. Therefore, `secp256k1_verify` costing 0.188 millisecond means it should cost $10^{10} \times 0.077 \times 10^{-3}$ = 770,000 gas.

This is comparable to CosmWasm's value.

### Batch signature verification

`ed25519_batch_verify` time for various batch sizes:

| Batch Size | Time (ms) |
| ---------- | --------- |
| 25         | 0.552     |
| 50         | 1.084     |
| 75         | 1.570     |
| 100        | 2.096     |
| 125        | 2.493     |
| 150        | 2.898     |

Linear regression shows there's a flat cost 0.134 ms (1,340,000 gas) plus 0.0188 ms (188,000 gas) per item.

### Hashes

Time (ms) for the host to perform hashes on inputs of various sizes:

| Hasher        | 200 kB | 400 kB | 600 kB | 800 kB | 1,000 kB | Gas Per Byte |
| ------------- | ------ | ------ | ------ | ------ | -------- | ------------ |
| `sha2_256`    | 0.544  | 1.086  | 1.627  | 2.201  | 2.718    | 27,265       |
| `sha2_512`    | 0.330  | 0.678  | 0.996  | 1.329  | 1.701    | 16,814       |
| `sha3_256`    | 0.298  | 0.606  | 0.918  | 1.220  | 1.543    | 15,326       |
| `sha3_512`    | 0.614  | 1.129  | 1.719  | 2.328  | 2.892    | 28,910       |
| `keccak256`   | 0.312  | 0.605  | 0.904  | 1.222  | 1.534    | 15,265       |
| `blake2s_256` | 0.305  | 0.632  | 0.907  | 1.212  | 1.526    | 15,244       |
| `blake2b_512` | 0.180  | 0.364  | 0.552  | 0.719  | 0.917    | 9,114        |
| `blake3`      | 0.105  | 0.221  | 0.321  | 0.411  | 0.512    | 5,195        |
