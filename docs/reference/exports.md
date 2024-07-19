# Entry points

Each Grug smart contract presents several predefined Wasm export functions known as **entry points**. The state machine (also referred to as the **host**) executes or makes queries at contracts by calling these functions. Some of the entry points are mandatory, while the others are optional. The Grug standard library provides an `#[grug_export]` macro which helps defining entry points.

This page lists all supported entry points, in _Rust pseudo-code_.

## Memory

These two are auto-implemented. They are used by the host to load data into the Wasm memory. The contract programmer should not try modifying them.

```rust
#[no_mangle]
extern "C" fn allocate(capacity: u32) -> u32;

#[no_mangle]
extern "C" fn deallocate(region_ptr: u32);
```

## Basic

These are basic entry points that pretty much every contract may need to implement.

```rust
#[grug_export]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> Result<Response, Error>;

#[grug_export]
fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> Result<Response, Error>;

#[grug_export]
fn migrate(ctx: MutableCtx, msg: MigrateMsg) -> Result<Response, Error>;

#[grug_export]
fn receive(ctx: MutableCtx) -> Result<Response, Error>;

#[grug_export]
fn reply(ctx: SudoCtx, msg: ReplyMsg, submsg_res: SubMsgResult) -> Result<Response, Error>;

#[grug_export]
fn query(ctx: ImmutableCtx, msg: QueryMsg) -> Result<Binary, Error>;
```

## Account

These are entry points that a contract needs in order to be able to initiate transactions.

```rust
#[grug_export]
fn before_tx(ctx: AuthCtx, tx: Tx) -> Result<Response, Error>;

#[grug_export]
fn after_tx(ctx: AuthCtx, tx: Tx) -> Result<Response, Error>;
```

## Cronjobs

The chain's owner can appoint a number of contracts to be automatically invoked at regular time intervals. Each such contract must implement the following entry point:

```rust
#[grug_export]
fn cron_execute(ctx: SudoCtx) -> Result<Response, Error>;
```

## Bank

These are mandatory entry points for the chain's **bank** contract.

```rust
#[grug_export]
fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> Result<Response, Error>;

#[grug_export]
fn bank_query(ctx: ImmutableCtx, msg: BankQuery) -> Result<BankQueryResponse, Error>;
```

## Gas

In Grug, gas fees are handled by a smart contract.

This contract is called after each transaction to collect gas fee from the sender. Develops can program arbitrary rules for collecting gas fees; for example, for an orderbook exchange, it may make sense to make the first few orders of each day free of charge, as a way to incentivize trading activity. Another use case is MEV capture. Osmosis is known to backrun certain DEX trades to perform arbitrage via its [ProtoRev module](https://github.com/osmosis-labs/osmosis/tree/main/x/protorev); this is something that can be realized using the gas contract, since it's automatically called after each transaction.

```rust
#[grug_export]
fn handle_fee(ctx: SudoCtx, report: GasReport) -> Result<Response>;
```

## IBC

Contracts that are to be used as IBC light clients must implement the following entry point:

```rust
#[grug_export]
fn ibc_client_query(ctx: ImmutableCtx, msg: IbcClientQuery) -> Result<()>;
```

Contracts that are to be used as IBC applications must implement the following entry points:

```rust
#[grug_export]
fn ibc_channel_open(ctx: MutableCtx, msg: IbcChannelOpenMsg) -> Result<Response>;

#[grug_export]
fn ibc_channel_close(ctx: MutableCtx, msg: IbcChannelCloseMsg) -> Result<Response>;

#[grug_export]
fn ibc_packet_receive(ctx: MutableCtx, msg: IbcPacketReceiveMsg) -> Result<Response>;

#[grug_export]
fn ibc_packet_ack(ctx: MutableCtx, msg: IbcPacketAckMsg) -> Result<Response>;

#[grug_export]
fn ibc_packet_timeout(ctx: MutableCtx, msg: IbcPacketTimeoutMsg) -> Result<Response>;
```
