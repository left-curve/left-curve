# grug-tester-immutable-state

This contract attempts to make state changes inside the `query`, `bank_query`, or `ibc_client_query` export functions.

This is typically impossible to do if using the Grug library in the intended way, where you're given an `ImmutableCtx` that contains an immutable `&dyn Storage`. However, a malicious contract can bypass this and call the FFI import functions (`db_write`, `db_remove`, and `db_remove_range`) directly.

We use this contract to test that the VMs can correctly reject attempts like this.

## License

TBD
