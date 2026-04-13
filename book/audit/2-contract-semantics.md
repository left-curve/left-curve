# Smart Contract Semantics

This chapter documents the programming model for Grug smart contracts: entry points,
context types, message passing, storage, authentication, and the testing framework.

## 1. Entry Points

Contracts export functions that the host calls at specific points in the transaction
lifecycle. Each entry point receives a typed context and returns a typed response.

### Basic entry points

| Entry Point | Context | Signature | Purpose |
|-------------|---------|-----------|---------|
| `instantiate` | `MutableCtx` | `fn(MutableCtx, M) -> Result<Response>` | One-time initialization on deploy |
| `execute` | `MutableCtx` | `fn(MutableCtx, M) -> Result<Response>` | State-mutating operations |
| `query` | `ImmutableCtx` | `fn(ImmutableCtx, M) -> Result<Binary>` | Read-only queries |
| `migrate` | `SudoCtx` | `fn(SudoCtx, M) -> Result<Response>` | Code upgrade migration |
| `receive` | `MutableCtx` | `fn(MutableCtx) -> Result<Response>` | Receive token transfers |
| `reply` | `SudoCtx` | `fn(SudoCtx, M, SubMsgResult) -> Result<Response>` | Callback after submessage |

### System entry points

| Entry Point | Context | Signature | Purpose |
|-------------|---------|-----------|---------|
| `authenticate` | `AuthCtx` | `fn(AuthCtx, Tx) -> Result<AuthResponse>` | Tx authentication (account contracts) |
| `backrun` | `AuthCtx` | `fn(AuthCtx, Tx) -> Result<Response>` | Post-tx hook (account contracts) |
| `withhold_fee` | `AuthCtx` | `fn(AuthCtx, Tx) -> Result<Response>` | Fee withholding (taxman only) |
| `finalize_fee` | `AuthCtx` | `fn(AuthCtx, Tx, TxOutcome) -> Result<Response>` | Fee settlement (taxman only) |
| `bank_execute` | `SudoCtx` | `fn(SudoCtx, BankMsg) -> Result<Response>` | Token ops (bank only) |
| `bank_query` | `ImmutableCtx` | `fn(ImmutableCtx, BankQuery) -> Result<BankQueryResponse>` | Balance queries (bank only) |
| `cron_execute` | `SudoCtx` | `fn(SudoCtx) -> Result<Response>` | Periodic automation |

Entry points are defined using the `#[grug::export]` attribute macro, which generates
the WASM FFI boilerplate (extern C functions, memory marshaling via `Region` structs).
This macro is only necessary when building contracts for the **WasmVm**. Contracts
targeting the **RustVm** (all first-party Dango contracts) do not need it -- they
register their entry points directly as Rust function pointers.

## 2. Context Types

Each entry point receives a context that controls what the contract can do.

```rust
// grug/types/src/context.rs

// Read-only access (queries)
pub struct ImmutableCtx<'a> {
    pub storage:  &'a dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
}

// Read-write access with sender and funds info (execute, instantiate)
pub struct MutableCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
    pub funds:    Coins,
}

// Read-write, chain-initiated (migrate, reply, cron_execute, bank)
pub struct SudoCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
}

// Authentication context (authenticate, backrun, withhold_fee, finalize_fee)
pub struct AuthCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub mode:     AuthMode,
}

pub enum AuthMode {
    Simulate,   // Gas estimation -- tx is unsigned (sig verify skipped)
    Check,      // CheckTx phase
    Finalize,   // FinalizeBlock phase
}
```

**Security note:** `MutableCtx` is the only context with `sender` and `funds`. A
`SudoCtx` entry point is called by the chain (no user sender). An `AuthCtx` entry
point knows which ABCI phase it's in, allowing it to skip signature verification
during simulation (the tx is not yet signed at that point -- the user needs the gas
estimate before they can sign).

## 3. Messages and Responses

### Transaction messages

A transaction contains a vector of `Message` variants:

```rust
pub enum Message {
    Configure(MsgConfigure),
    Upgrade(MsgUpgrade),
    Transfer(MsgTransfer),
    Upload(MsgUpload),
    Instantiate(MsgInstantiate),
    Execute(MsgExecute),
    Migrate(MsgMigrate),
}

pub struct MsgExecute {
    pub contract: Addr,
    pub msg: Json,
    pub funds: Coins,
}
```

### Contract responses

```rust
pub struct Response {
    pub submsgs: Vec<SubMessage>,
    pub subevents: Vec<ContractEvent>,
}

pub struct AuthResponse {
    pub response: Response,
    pub request_backrun: bool,  // Whether to call backrun() after tx finalization
}
```

### Submessages and replies

Contracts can emit submessages -- nested calls that execute after the current entry
point returns:

```rust
pub struct SubMessage {
    pub msg: Message,
    pub reply_on: ReplyOn,
}

pub enum ReplyOn {
    Success(Json),   // Reply only on success (payload passed to reply())
    Error(Json),     // Reply only on failure
    Always(Json),    // Reply regardless
    Never,           // No reply callback
}

pub type SubMsgResult = Result<Event, String>;
```

**Execution semantics:**

| `reply_on` | Submsg succeeds | Submsg fails | Submsg state on failure |
|------------|-----------------|--------------|-------------------------|
| `Success` | Call `reply()` | **Abort entire tx** | **Reverted** (entire tx) |
| `Error` | Do nothing | Call `reply()` | **Reverted** |
| `Always` | Call `reply()` | Call `reply()` | **Reverted** |
| `Never` | Do nothing | **Abort entire tx** | **Reverted** (entire tx) |

Each submessage executes in its own `Buffer`. On success, the buffer is committed to
the parent. **On failure, the submessage's state changes are always reverted** (its
buffer is discarded). If `reply_on` is `Error` or `Always`, the parent continues and
`reply()` is called; otherwise, the entire transaction is aborted.

**Security implication:** A failed submessage can never leave behind partial state
changes. If no reply handler catches the failure, the entire transaction is aborted,
preventing contracts from silently ignoring errors.

## 4. Core Types

### Addresses

```rust
pub type Addr = EncodedBytes<[u8; 20], AddrEncoder>;  // 20-byte, 0x-prefixed hex

// Deterministic address derivation
// address = ripemd160(sha256(deployer_addr || code_hash || salt))
```

All `Addr` fields are validated during deserialization. Invalid hex or wrong length is
rejected before contract code runs.

### Coins

```rust
pub type Coins = BTreeMap<Denom, Uint128>;

// Ordered, deduplicated, non-zero amounts enforced
```

### Math types

`grug/math/` provides overflow-safe fixed-point arithmetic:

| Type | Description |
|------|-------------|
| `Uint128`, `Uint256` | Unsigned integers |
| `Int128`, `Int256` | Signed integers |
| `Udec128`, `Udec256` | Unsigned decimals (18 decimal places) |
| `Dec128`, `Dec256` | Signed decimals (18 decimal places) |

All arithmetic is checked. Overflow/underflow returns `StdError` instead of panicking.

### Dimensional Number type

Dango extends the base math types with `Number<Q, U, D>`
(`dango/types/src/typed_number.rs`), a **dimensionally-typed** signed fixed-point
decimal (`Dec128_6` -- 6 decimal places). The three type parameters encode physical
dimensions using `typenum` integers:

- **Q** -- quantity (asset units)
- **U** -- USD value
- **D** -- time duration (days)

Multiplication and division **propagate dimensions at the type level**, so the
compiler rejects nonsensical operations (e.g., adding a price to a quantity):

```rust
// price × quantity = USD value  (Q: -1+1=0, U: 1+0=1, D: 0+0=0)
fn checked_mul<Q1, U1, D1>(self, rhs: Number<Q1, U1, D1>)
    -> MathResult<Number<Q + Q1, U + U1, D + D1>>;

// USD value / price = quantity  (Q: 0-(-1)=1, U: 1-1=0, D: 0-0=0)
fn checked_div<Q1, U1, D1>(self, rhs: Number<Q1, U1, D1>)
    -> MathResult<Number<Q - Q1, U - U1, D - D1>>;
```

Key type aliases used throughout the perps and DEX contracts:

| Alias | Dimensions (Q, U, D) | Meaning |
|-------|----------------------|---------|
| `Dimensionless` | (0, 0, 0) | Pure scalar (ratios, rates) |
| `Quantity` | (1, 0, 0) | Asset amount in human units |
| `UsdValue` | (0, 1, 0) | Dollar amount |
| `UsdPrice` | (-1, 1, 0) | Price (USD per unit of asset) |
| `FundingPerUnit` | (-1, 1, 0) | Cumulative funding accumulator |
| `FundingRate` | (0, 0, -1) | Funding rate (per day) |
| `Days` | (0, 0, 1) | Time duration in days |

This type system is a key defense against unit-confusion bugs in margin, PnL, and
funding calculations. A mismatched dimension is a compile-time error, not a runtime
surprise.

### Bounded types

Grug encourages declarative validation via `Bounded<T, B>` and `LengthBounded<T>`:

```rust
struct FeeRateBounds;
impl Bounds<Udec256> for FeeRateBounds {
    const MIN: Bound<Udec256> = Bound::Inclusive(Udec256::ZERO);
    const MAX: Bound<Udec256> = Bound::Exclusive(Udec256::ONE);
}
type FeeRate = Bounded<Udec256, FeeRateBounds>;

// Length bounds
pub type Label = LengthBounded<String, 1, 128>;
pub type Salt = LengthBounded<Binary, 1, 82>;
```

Bounds are enforced during deserialization -- contracts never see out-of-bounds data.

## 5. Storage Abstractions

### Item (single value)

```rust
const CONFIG: Item<Config> = Item::new("config");

CONFIG.save(storage, &value)?;
let v = CONFIG.load(storage)?;
let v = CONFIG.may_load(storage)?;  // Option<T>
```

### Map (key-value)

```rust
const BALANCES: Map<Addr, Uint128> = Map::new("balances");

BALANCES.save(storage, addr, &amount)?;
let amt = BALANCES.load(storage, addr)?;
BALANCES.has(storage, addr);
BALANCES.remove(storage, addr);

// Iteration
for (key, value) in BALANCES.range(storage, None, None, Order::Ascending)? {
    // ...
}
```

### Set (membership)

```rust
const WHITELIST: Set<Addr> = Set::new("whitelist");

WHITELIST.insert(storage, addr)?;
WHITELIST.has(storage, addr);
WHITELIST.remove(storage, addr);
```

### Counter

```rust
const NONCE: Counter<u32> = Counter::new("nonce", 0, 1);  // base=0, step=1

let (old, new) = NONCE.increment(storage)?;
```

### IndexedMap

For queryable maps with secondary indexes:

```rust
const USERS: IndexedMap<UserIndex, User, UserIndexes> = IndexedMap::new("user", indexes);

// Primary key access
USERS.save(storage, user_idx, &user)?;
let user = USERS.load(storage, user_idx)?;

// Secondary index queries
USERS.idx.by_account.prefix(addr).range(...)?;
USERS.idx.by_name.prefix(name).range(...)?;
```

Index types:
- `MultiIndex<PK, IK, T>` -- one primary key can map to many index keys (one-to-many).
- `UniqueIndex<PK, IK, T>` -- one primary key maps to exactly one unique index key.

## 6. Cross-Contract Communication

### Queries

Contracts can query other contracts or chain state via the `QuerierWrapper`:

```rust
// Query another contract's custom endpoint
let result: R::Response = ctx.querier.query_wasm_smart(contract_addr, query_msg)?;

// Query raw storage of another contract
let raw: Option<Binary> = ctx.querier.query_wasm_raw(contract_addr, key)?;

// Query bank balances
let balance: Coin = ctx.querier.query_balance(addr, denom)?;
let all: Coins = ctx.querier.query_balances(addr)?;
```

Queries are read-only and gas-metered. They cannot mutate state. Recursive queries
are limited to depth 3 to prevent stack overflow.

### Submessages (state-mutating calls)

To call another contract with state mutation, return submessages in the `Response`:

```rust
let msg = Message::execute(target_addr, &call_msg, coins)?;
let response = Response::new()
    .add_message(msg)                          // reply_on: Never
    .add_submessage(SubMessage::reply_on_success(msg, &data)?);  // reply_on: Success
```

## 7. Authentication and Account Model

Grug uses **account abstraction** -- every user has a dedicated smart contract instance
that handles authentication.

### Account lifecycle

1. **Registration.** User calls the account factory with a signed `RegisterUser`
   message.
2. **Account creation.** The factory deploys an account contract instance, registers
   the user's public key, and optionally activates the account.
3. **Transaction signing.** User constructs a `SignDoc` (sender, messages, nonce,
   expiry), signs it, and submits a `Tx`.
4. **Authentication.** The host calls the account contract's `authenticate()` entry
   point. The contract verifies the signature, nonce, and account status.

### Nonce management

Instead of a strictly incrementing nonce (which forces sequential tx ordering), Dango
tracks the **most recent 20 nonces seen** (`SEEN_NONCES`). A new tx is accepted if:

- Its nonce is not already in `SEEN_NONCES`.
- Its nonce is greater than the smallest nonce in `SEEN_NONCES`.
- Its nonce does not jump more than 100 from the current maximum.

This allows concurrent, unordered transaction submission. See
[Nonces and unordered transactions](../notes/nonces.md) for details.

### Account status

```rust
pub enum AccountStatus {
    Inactive,  // Not yet funded or activated
    Active,    // Can send transactions
    Frozen,    // Blocked (e.g., by governance)
}
```

Inactive accounts are activated on sufficient deposit (≥ `min_deposit` from app config).

### Signature types

| Type | Curve | Use case |
|------|-------|----------|
| `Passkey` | Secp256r1 | WebAuthn / browser passkeys |
| `Secp256k1` | Secp256k1 | Standard crypto wallets |
| `Eip712` | Secp256k1 | Ethereum wallet compatibility |

## 8. FFI Layer

`grug/ffi/` bridges WASM guests and the host:

- **Exports** (`ffi/src/exports.rs`): `do_instantiate`, `do_execute`, `do_query`, etc.
  These deserialize context and message from WASM memory, call the contract function,
  and serialize the result back.
- **Imports** (`ffi/src/imports.rs`): `db_read`, `db_write`, `secp256k1_verify`, etc.
  These are extern C functions the guest calls to invoke host capabilities.
- **Memory** (`ffi/src/memory.rs`): Uses `Region` structs (offset + capacity) to
  describe buffers in WASM linear memory. `allocate` and `deallocate` are auto-provided
  entry points.

## 9. Testing Framework

### TestSuite

`grug/testing/` provides a high-level integration test harness:

```rust
let suite = TestBuilder::new()
    .with_chain_id("test-chain")
    .with_block_time(Duration::from_secs(5))
    .with_genesis_state(genesis)?
    .build()?;

// Upload and deploy a contract
suite.upload(wasm_code)?;
let addr = suite.instantiate(code_hash, &msg, None)?;

// Execute and query
let outcome = suite.execute(addr, &execute_msg, &funds)?;
let result: QueryResponse = suite.query(addr, &query_msg)?;

// Advance blocks
suite.make_block()?;
```

### Test helpers

- `outcome.should_succeed()` / `outcome.should_fail()` -- Assert tx result.
- `outcome.should_fail_with_error("msg")` -- Assert specific error.
- Event inspection via `outcome.events`.

### Testing with MemDb + RustVm

Tests use `MemDb` (in-memory, no disk I/O) and `RustVm` (native execution, no WASM
compilation). This makes tests fast and deterministic while exercising the same
storage and execution paths as production.

### Dango-specific test suite

`dango/testing/` extends the base suite with helpers for deploying the full Dango
contract system (bank, taxman, accounts, oracle, DEX, perps) in a single genesis
block. This enables end-to-end tests that exercise inter-contract interactions.

## 10. Procedural Macros

`grug/macros/` provides:

- **`#[grug::export]`** -- Generates WASM FFI wrappers for entry points. Only needed
  for WasmVm contracts; RustVm contracts register entry points directly.
- **`#[grug::derive(Serde, Borsh)]`** -- Derives standard traits (Serialize,
  Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq).
- **`#[grug::event("name")]`** -- Registers an event type with a canonical name.
- **`#[grug::index_list(PK, T)]`** -- Implements `IndexList` trait for IndexedMap
  secondary indexes.
