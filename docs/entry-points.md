# Entry points

Each CWD smart contract presents several predefined Wasm export functions known as **entry points**. The state machine (also referred to as the **host**) executes or makes queries at contracts by calling these functions. Some of the entry points are mandatory, while the others are optional. The CWD standard library provides an `#[entry_point]` macro which helps defining entry points.

This page lists all supported entry points, in _Rust pseudo-code_.

## Memory

These two are automatically implemented by the cw-std library. They are used by the host to load data into the Wasm memory. The contract programmer should not try modifying them.

```rust
#[no_mangle]
extern "C" fn allocate(capacity: u32) -> u32;

#[no_mangle]
extern "C" fn deallocate(region_ptr: u32);
```

## Basic

These are basic entry points that pretty much every contract may need to implement.

```rust
#[entry_point]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> Result<Response, Error>;

#[entry_point]
fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> Result<Response, Error>;

#[entry_point]
fn migrate(ctx: MutableCtx, msg: MigrateMsg) -> Result<Response, Error>;

#[entry_point]
fn receive(ctx: MutableCtx) -> Result<Response, Error>;

#[entry_point]
fn reply(ctx: SudoCtx, msg: ReplyMsg, submsg_res: SubMsgResult) -> Result<Response, Error>;

#[entry_point]
fn query(ctx: ImmutableCtx, msg: QueryMsg) -> Result<Binary, Error>;
```

## Account

These are entry points that a contract needs in order to be able to initiate transactions.

```rust
#[entry_point]
fn before_tx(ctx: AuthCtx, tx: Tx) -> Result<Response, Error>;

#[entry_point]
fn after_tx(ctx: AuthCtx, tx: Tx) -> Result<Response, Error>;
```

## Cronjobs

Each chain can optionally have one _begin blocker_ contract and an _end blocker_ contract. The following entry points of these two contract are called once at the beginning and end of each block. This is useful if there are actions that need to be performed at regular intervals, such as for a perpetual futures protocol, updating the funding rate parameters.

```rust
#[entry_point]
fn before_block(ctx: SudoCtx) -> Result<Response, Error>;

#[entry_point]
fn after_block(ctx: SudoCtx) -> Result<Response, Error>;
```

## Bank

These are mandatory entry points for the chain's **bank** contract.

```rust
#[entry_point]
fn bank_transfer(ctx: SudoCtx, msg: BankTransferMsg) -> Result<Response, Error>;

#[entry_point]
fn bank_query(ctx: ImmutableCtx, msg: BankQueryMsg) -> Result<BankQueryResponse, Error>;
```

## Gas

In CWD, gas fees are handled by a smart contract.

This contract is called after each transaction to collect gas fee from the sender. Develops can program arbitrary rules for collecting gas fees; for example, for an orderbook exchange, it may make sense to make the first few orders of each day free of charge, as a way to incentivize trading activity. Another use case is MEV capture. Osmosis is known to backrun certain DEX trades to perform arbitrage via its [ProtoRev module](https://github.com/osmosis-labs/osmosis/tree/main/x/protorev); this is something that can be realized using the gas contract, since it's automatically called after each transaction.

```rust
#[entry_point]
fn handle_fee(ctx: SudoCtx, report: GasReport) -> Result<Response>;
```

## IBC

Contracts that are to be used as IBC light clients must implement the following entry points:

```rust
/// Called upon client creation. It allows the client to perform validation on
/// the initial consensus state and set the client state, consensus state, and
/// any client-specific metadata necessary for correct light client operation.
#[entry_point]
fn ibc_client_create(ctx: SudoCtx, msg: IbcClientCreateMsg) -> Result<Response>;

#[entry_point]
fn ibc_client_execute(ctx: SudoCtx, msg: IbcClientExecuteMsg) -> Result<Response>;

#[entry_point]
fn ibc_client_query(ctx: ImmutableCtx, msg: IbcClientQueryMsg) -> Result<IbcClientQueryResponse>;
```

Contracts that are to be used as IBC applications must implement the following entry points:

```rust
#[entry_point]
fn ibc_channel_open(ctx: SudoCtx, msg: IbcChannelOpenMsg) -> Result<Response>;

#[entry_point]
fn ibc_channel_close(ctx: SudoCtx, msg: IbcChannelCloseMsg) -> Result<Response>;

#[entry_point]
fn ibc_packet_receive(ctx: SudoCtx, msg: IbcPacketReceiveMsg) -> Result<Response>;

#[entry_point]
fn ibc_packet_ack(ctx: SudoCtx, msg: IbcPacketAckMsg) -> Result<Response>;

#[entry_point]
fn ibc_packet_timeout(ctx: SudoCtx, msg: IbcPacketTimeoutMsg) -> Result<Response>;
```
