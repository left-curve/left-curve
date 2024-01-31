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
fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> Result<Response, Error>;

#[entry_point]
fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> Result<Response, Error>;

#[entry_point]
fn query(ctx: QueryCtx, msg: QueryMsg) -> Result<Binary, Error>;

#[entry_point]
fn migrate(ctx: MigrateCtx, msg: MigrateMsg) -> Result<Response, Error>;

#[entry_point]
fn reply(ctx: ReplyCtx, msg: ReplyMsg) -> Result<Response, Error>;

#[entry_point]
fn receive(ctx: ReceiveCtx) -> Result<Response, Error>;
```

## Account

These are entry points that a contract needs in order to be able to initiate transactions.

```rust
#[entry_point]
fn before_tx(ctx: BeforeTxCtx, tx: Tx) -> Result<Response, Error>;

#[entry_point]
fn after_tx(ctx: AfterTxCtx) -> Result<Response, Error>;
```

## Bank

These are mandatory entry points for the chain's **bank** contract.

```rust
#[entry_point]
fn transfer(ctx: TransferCtx, msg: TransferMsg) -> Result<Response, Error>;

#[entry_point]
fn query_bank(ctx: QueryCtx, msg: BankQueryMsg) -> Result<BankQueryResponse, Error>;
```
