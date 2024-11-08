# Transaction lifecycle

A Grug transaction (tx) is defined by the following struct:

```rust
struct Tx {
    pub sender: Addr,
    pub gas_limit: u64,
    pub msgs: Vec<Message>,
    pub data: Json,
    pub credential: Json,
}
```

Explanation of the fields:

#### Sender

The account that sends this tx, who will perform authentication and (usually) pay the tx fee.

#### Gas limit

The maximum amount of gas requested for executing this tx.

If gas of this amount is exhausted at any point, execution is aborted and state changes discarded.[^1]

#### Messages

A list of `Message`s to be executed.

They are executed in the specified order and _atomically_, meaning they either succeed altogether, or fail altogether; a single failed message failing leads to the entire tx aborted.[^2]

#### Data

Auxilliary data to attach to the tx.

An example use case of this is if the chain accepts multiple tokens for fee payment, the sender can specify here which denom to use:

```json
{
  "data": {
    "fee_denom": "uatom"
  }
}
```

The **taxman** contract, which handles fees, should be programmed to deserialize this data, and use appropriate logics to handle the fee (e.g. swap the tokens on a DEX).

#### Credential

An arbitrary data to prove the tx was composed by the rightful owner of the sender account. Most commonly, this is a cryptographic signature.

Note that `data` is an opaque `grug::Json` (which is an alias to `serde_json::Value`) instead of a concrete type. This is because Grug does not attempt to intrepret or do anything about the credential. It's all up to the sender account. Different accounts may expect different credential types.

Next we discuss the full lifecycle of a transaction.

## Simulation

The user have specified `sender`, `msgs`, and `data` fields by interacting with a webapp. The next step now is to determine an appropriate `gas_limit`.

For some simple txs, we can make a reasonably good guess of gas consumption. For example, a tx consisting of a single `Message::Transfer` of a single coin should consume just under 1,000,000 gas (of which 770,000 is for Secp256k1 signature verification).

However, for more complex txs, it's necessary to query a node to **simulate** its gas consumption.

To do this, compose an `UnsignedTx` value:

```rust
struct UnsignedTx {
    pub sender: Addr,
    pub msgs: Vec<Message>,
    pub data: Json,
}
```

which is basically `Tx` but lacks the `gas_limit` and `credential` fields.

Then, invoke the ABCI `Query` method with the string `"/simulate"` as `path`:

- Using the Rust SDK, this can be done with the `grug_sdk::Client::simulate` method.
- Using the CLI, append the `--simulate` to the `tx` subcommand.

The `App` will run _the entire tx_ in **simulation mode**, and return an `Outcome` value:

```rust
struct Outcome {
    pub gas_limit: Option<u64>,
    pub gas_used: u64,
    pub result: GenericResult<Vec<Event>>,
}
```

This includes the amount of gas used, and if the tx succeeded, the events that were emitted; or, in case the tx failed, the error message.

Two things to note:

- In simulation mode, certain steps in authentication are skipped, such as signature verification (we haven't signed the tx yet at this point). This means gas consumption is underestimated. Since we know an Secp256k1 verification costs 770,000 gas, it's advisable to add this amount manually.
- The max amount of gas the simulation can consume is the node's query gas limit, which is an offchain parameter chosen individually by each node. If the node has a low query gas limit (e.g. if the node is not intended to serve heavy query requests), the simulation may fail.

## CheckTx

Now we know the gas limit, the user will sign the tx, and we create the `Tx` value and broadcast it to a node.

Tendermint will now call the ABCI `CheckTx` method, and decide whether to accept the tx into mempool or not, based on the result.

When serving a `CheckTx` request, the `App` doesn't execute the entire tx. This is because while some messages may fail at this time, they may succeed during `FinalizeBlock`, as the chain's state would have changed.

Therefore, instead, the `App` only performs the first two steps:

1. Call the taxman's `withhold_fee` method. This ensures the tx's sender has enough fund to afford the tx fee.
2. Call the sender's `authenticate` method in normal (i.e. non-simulation) mode. Here the sender performs authentication (which is skipped in simulation mode).

Tendermint will reject the tx if `CheckTx` fails (meaning, either `withfold_fee` or `authenticate` fails), or if the tx's gas limit is bigger than the block gas limit (it can't fit in a block). Otherwise, it's inserted into the mempool.

## FinalizeBlock

In `FinalizeBlock`, the entire tx processing flow is performed, which is:

1. Call taxman's `withhold_fee` method.

   This MUST succeed (if it would fail, it should have failed during `CheckTx` such that the tx is rejected from entering mempool). If does fail for some reason (e.g. a previous tx in the block drained the sender's wallet, so it can no longer affored the fee), the processing is aborted and all state changes discarded.
2. Call sender's `authenticate` method.

   If fails, discard state changes from step 2 (keeping those from step 1), then jump to step 5.

3. Loop through the messages, execute one by one.

   If any fails, discard state changes from step 2-3, then jump to step 5.
4. Call sender's `backrun` method.

   If fails, discard state changes from step 2-4, then jump to step 5.
5. Call taxman's `finalize_fee` method.

   This MUST succeed (the bank and taxman contracts should be programmed in a way that ensures this). If it does fail for some reason, discard all state changes for all previous steps and abort.

> TODO: make a flow chart

## Summary

|                            | Simulate                | CheckTx             | FinalizeBlock       |
| -------------------------- | ----------------------- | ------------------- | ------------------- |
| Input type                 | `UnsignedTx`            | `Tx`                | `Tx`                |
| Call taxman `withhold_fee` | Yes                     | Yes                 | Yes                 |
| Call sender `authenticate` | Yes, in simulation mode | Yes, in normal mode | Yes, in normal mode |
| Execute messages           | Yes                     | No                  | Yes                 |
| Call sender `backrun`      | Yes                     | No                  | Yes                 |
| Call taxman `finalize_fee` | Yes                     | No                  | Yes                 |

[^1]: Transaction fee is still deducted. See the discussion on fee handling later in the article.

[^2]: This said, a `SubMessage` can fail without aborting the tx, if it's configured as such (with `SubMessage::reply_on` set to `ReplyOn::Always` or `ReplyOn::Error`).
