# Entry points

Each Grug smart contract presents several predefined Wasm export functions known as **entry points**. The state machine (also referred to as the **host**) executes or makes queries at contracts by calling these functions. Some of the entry points are mandatory, while the others are optional. The Grug standard library provides an `#[grug::export]` macro which helps defining entry points.

This page lists all supported entry points, in _Rust pseudo-code_.

## Memory

These two are auto-implemented. They are used by the host to load data into the Wasm memory. The contract programmer should not try modifying them.

```rust
#[unsafe(no_mangle)]
extern "C" fn allocate(capacity: u32) -> u32;

#[unsafe(no_mangle)]
extern "C" fn deallocate(region_ptr: u32);
```

## Basic

These are basic entry points that pretty much every contract may need to implement.

```rust
#[grug::export]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> Result<Response>;

#[grug::export]
fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> Result<Response>;

#[grug::export]
fn migrate(ctx: MutableCtx, msg: MigrateMsg) -> Result<Response>;

#[grug::export]
fn receive(ctx: MutableCtx) -> Result<Response>;

#[grug::export]
fn reply(ctx: SudoCtx, msg: ReplyMsg, result: SubMsgResult) -> Result<Response>;

#[grug::export]
fn query(ctx: ImmutableCtx, msg: QueryMsg) -> Result<Binary>;
```

## Fee

In Grug, gas fees are handled by a smart contract called the **taxman**. It must implement the following two exports:

```rust
#[grug::export]
fn withhold_fee(ctx: AuthCtx, tx: Tx) -> Result<Response>;

#[grug::export]
fn finalize_fee(ctx: AuthCtx, tx: Tx, outcome: Outcome) -> Result<Response>;
```

## Authentication

These are entry points that a contract needs in order to be able to initiate transactions.

```rust
#[grug::export]
fn authenticate(ctx: AuthCtx, tx: Tx) -> Result<Response>;

#[grug::export]
fn backrun(ctx: AuthCtx, tx: Tx) -> Result<Response>;
```

## Bank

In Grug, tokens balances and transfers are handled by a contract known as the **bank**. It must implement the following two exports:

```rust
#[grug::export]
fn bank_execute(ctx: SudoCtx, msg: BankMsg) -> Result<Response>;

#[grug::export]
fn bank_query(ctx: ImmutableCtx, msg: BankQuery) -> Result<BankQueryResponse>;
```

## Cronjobs

The chain's owner can appoint a number of contracts to be automatically invoked at regular time intervals. Each such contract must implement the following entry point:

```rust
#[grug::export]
fn cron_execute(ctx: SudoCtx) -> Result<Response>;
```

## IBC

Contracts that are to be used as IBC light clients must implement the following entry point:

```rust
#[grug::export]
fn ibc_client_query(ctx: ImmutableCtx, msg: IbcClientQuery) -> Result<IbcClientQueryResponse>;
```

Contracts that are to be used as IBC applications must implement the following entry points:

```rust
#[grug::export]
fn ibc_packet_receive(ctx: MutableCtx, msg: IbcPacketReceiveMsg) -> Result<Response>;

#[grug::export]
fn ibc_packet_ack(ctx: MutableCtx, msg: IbcPacketAckMsg) -> Result<Response>;

#[grug::export]
fn ibc_packet_timeout(ctx: MutableCtx, msg: IbcPacketTimeoutMsg) -> Result<Response>;
```
