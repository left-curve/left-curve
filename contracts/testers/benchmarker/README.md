# grug-tester-benchmarker

This contract exposes a single query function `loop`, which runs a loop of the given number of iterations, each iteration containing a set of math operations (additions, subtractions, multiplications, divisions).

By benchmarking this, we can establish the relation between Wasm operations and CPU time, i.e. how many operations can be performed per one second of time. This way, we can assign a gas value for the operations.

E.g. We target `X` gas units per second; we measure that our `WasmVm` executes `Y` operations per second. As such, each operation should cost `X / Y` gas units.

Code for this benchmark can be found in [crates/vm/wasm/benches](../../../crates/vm/wasm/benches/).
