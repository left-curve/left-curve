# API Reference

This chapter documents the complete API for the Dango perpetual futures exchange. It is intended as a standalone reference for building SDKs and trading systems. All interactions with the chain go through a single **GraphQL endpoint** that supports queries, mutations, and WebSocket subscriptions.

There are two query surfaces:

- **On-chain queries** — read contract state directly via the `queryApp` GraphQL field. Returns the latest finalized state.
- **Indexer queries** — read historical and aggregated data (trade history, candlesticks, 24h stats) via dedicated GraphQL fields backed by a time-series database.

Both surfaces share the same endpoint. All write operations (orders, deposits, account creation) go through the `broadcastTxSync` GraphQL mutation.

## 1. Transport

### 1.1 HTTP

All queries and mutations use a standard GraphQL POST request.

**Endpoint:** `https://<host>/graphql`

**Headers:**

| Header         | Value              |
| -------------- | ------------------ |
| `Content-Type` | `application/json` |

**Request body:**

```json
{
  "query": "query { ... }",
  "variables": { ... }
}
```

**Example — query chain status:**

```bash
curl -X POST https://<host>/graphql \
  -H 'Content-Type: application/json' \
  -d '{"query": "{ queryStatus { block { blockHeight timestamp } chainId } }"}'
```

**Response:**

```json
{
  "data": {
    "queryStatus": {
      "block": {
        "blockHeight": 123456,
        "timestamp": "2026-01-15T12:00:00"
      },
      "chainId": "dango-1"
    }
  }
}
```

A **GraphiQL** playground is available at the same URL via HTTP GET.

### 1.2 WebSocket

Subscriptions (real-time data) use WebSocket with the `graphql-ws` protocol.

**Endpoint:** `wss://<host>/graphql`

**Connection handshake:**

```json
{"type": "connection_init", "payload": {}}
```

**Subscribe:**

```json
{
  "id": "1",
  "type": "subscribe",
  "payload": {
    "query": "subscription { perpsTrades(pairId: \"perp/btcusd\") { fillPrice fillSize } }"
  }
}
```

**Messages arrive as:**

```json
{"id": "1", "type": "next", "payload": {"data": {"perpsTrades": { ... }}}}
```

### 1.3 Pagination

List queries use **cursor-based pagination** (Relay Connection specification).

| Parameter | Type     | Description                               |
| --------- | -------- | ----------------------------------------- |
| `first`   | `Int`    | Return the first N items                  |
| `after`   | `String` | Cursor — return items after this          |
| `last`    | `Int`    | Return the last N items                   |
| `before`  | `String` | Cursor — return items before this         |
| `sortBy`  | `Enum`   | `BLOCK_HEIGHT_ASC` or `BLOCK_HEIGHT_DESC` |

**Response shape:**

```json
{
  "pageInfo": {
    "hasNextPage": true,
    "hasPreviousPage": false,
    "startCursor": "abc...",
    "endCursor": "xyz..."
  },
  "nodes": [ ... ]
}
```

Use `first` + `after` for forward pagination, `last` + `before` for backward.

## 2. Authentication and transactions

### 2.1 Transaction structure

Every write operation is wrapped in a signed **transaction** (`Tx`):

```json
{
  "sender": "0x1234...abcd",
  "gas_limit": 1500000,
  "msgs": [
    {
      "execute": {
        "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
        "msg": { ... },
        "funds": {}
      }
    }
  ],
  "data": { ... },
  "credential": { ... }
}
```

| Field        | Type         | Description                                          |
| ------------ | ------------ | ---------------------------------------------------- |
| `sender`     | `Addr`       | Account address sending the transaction              |
| `gas_limit`  | `u64`        | Maximum gas units for execution                      |
| `msgs`       | `[Message]`  | Non-empty list of messages to execute atomically     |
| `data`       | `Metadata`   | Replay protection metadata (see [2.2](#22-metadata)) |
| `credential` | `Credential` | Cryptographic proof of sender authorization          |

Messages execute **atomically** — either all succeed or all fail.

### 2.2 Metadata

The `data` field contains replay protection metadata:

```json
{
  "user_index": 0,
  "chain_id": "dango-1",
  "nonce": 42,
  "expiry": null
}
```

| Field        | Type                | Description                                                       |
| ------------ | ------------------- | ----------------------------------------------------------------- |
| `user_index` | `u32`               | The user index that owns the sender account                       |
| `chain_id`   | `String`            | Chain identifier (prevents cross-chain replay)                    |
| `nonce`      | `u32`               | Replay protection nonce                                           |
| `expiry`     | `Timestamp \| null` | Optional expiration (nanoseconds since epoch); `null` = no expiry |

**Nonce semantics.** Dango uses **unordered nonces** with a sliding window of 20. The account tracks the 20 most recently seen nonces. A transaction is accepted if its nonce is newer than the oldest seen nonce and has not been used before. This means transactions may arrive out of order without being rejected. SDK implementations should track the next available nonce client-side by querying the account's seen nonces and choosing the next integer above the maximum.

### 2.3 Message format

The primary message type for interacting with contracts is `execute`:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "0.100000",
          "kind": { "market": { "max_slippage": "0.010000" } },
          "reduce_only": false
        }
      }
    },
    "funds": {}
  }
}
```

| Field      | Type    | Description                                                                |
| ---------- | ------- | -------------------------------------------------------------------------- |
| `contract` | `Addr`  | Target contract address                                                    |
| `msg`      | `JSON`  | Contract-specific execute message (snake_case keys)                        |
| `funds`    | `Coins` | Tokens to send with the message: `{"<denom>": "<amount>"}` or `{}` if none |

The `funds` field is a map of denomination to amount string. For example, depositing 1000 USDC:

```json
"funds": { "usdc": "1000000000" }
```

USDC uses **6 decimal places** in its base unit (1 USDC = `1000000` base units).

### 2.4 Signing methods

The `credential` field wraps a `StandardCredential` or `SessionCredential`. A `StandardCredential` identifies the signing key and contains the signature:

**Passkey (Secp256r1 / WebAuthn):**

```json
{
  "standard": {
    "key_hash": "a1b2c3d4...64hex",
    "signature": {
      "passkey": {
        "authenticator_data": "<base64>",
        "client_data": "<base64>",
        "sig": "0102...40hex"
      }
    }
  }
}
```

- `sig`: 64-byte Secp256r1 signature (hex-encoded)
- `client_data`: base64-encoded WebAuthn client data JSON (challenge = base64url of SHA-256 of SignDoc)
- `authenticator_data`: base64-encoded WebAuthn authenticator data

**Secp256k1:**

```json
{
  "standard": {
    "key_hash": "a1b2c3d4...64hex",
    "signature": {
      "secp256k1": "0102...40hex"
    }
  }
}
```

- 64-byte Secp256k1 signature (hex-encoded)

**EIP-712 (Ethereum wallets):**

```json
{
  "standard": {
    "key_hash": "a1b2c3d4...64hex",
    "signature": {
      "eip712": {
        "typed_data": "<base64>",
        "sig": "0102...41hex"
      }
    }
  }
}
```

- `sig`: 65-byte signature (64-byte Secp256k1 + 1-byte recovery ID; hex-encoded)
- `typed_data`: base64-encoded JSON of the EIP-712 typed data object

### 2.5 Session credentials

Session keys allow delegated signing without requiring the master key for every transaction.

```json
{
  "session": {
    "session_info": {
      "session_key": "02abc...33bytes",
      "expire_at": "1700000000000000000"
    },
    "session_signature": "0102...40hex",
    "authorization": {
      "key_hash": "a1b2c3d4...64hex",
      "signature": { ... }
    }
  }
}
```

| Field               | Type                 | Description                                     |
| ------------------- | -------------------- | ----------------------------------------------- |
| `session_info`      | `SessionInfo`        | Session key public key + expiration             |
| `session_signature` | `ByteArray<64>`      | SignDoc signed by the session key (hex-encoded) |
| `authorization`     | `StandardCredential` | SessionInfo signed by the user's master key     |

### 2.6 SignDoc

The **SignDoc** is the data structure that gets signed. It mirrors the transaction but replaces the `credential` with the structured `Metadata`:

```json
{
  "data": {
    "chain_id": "dango-1",
    "expiry": null,
    "nonce": 42,
    "user_index": 0
  },
  "gas_limit": 1500000,
  "messages": [ ... ],
  "sender": "0x1234...abcd"
}
```

**Signing process:**

1. Serialize the SignDoc to **canonical JSON** (fields sorted alphabetically).
2. Hash the serialized bytes with **SHA-256**.
3. Sign the hash with the appropriate key.

For Passkey (WebAuthn), the SHA-256 hash becomes the `challenge` in the WebAuthn request. For EIP-712, the SignDoc is mapped to an EIP-712 typed data structure and signed via `eth_signTypedData_v4`.

### 2.7 Signing flow

The full transaction lifecycle:

1. **Compose messages** — build the contract execute message(s).
2. **Fetch metadata** — query chain ID, account's user_index, and next available nonce.
3. **Simulate** — send an `UnsignedTx` to estimate gas (see [2.8](#28-gas-estimation)).
4. **Set gas limit** — use the simulation result, adding ~770,000 for signature verification overhead.
5. **Build SignDoc** — assemble `{sender, gas_limit, messages, data}`.
6. **Sign** — sign the SignDoc with the chosen method.
7. **Broadcast** — submit the signed `Tx` via `broadcastTxSync` (see [2.9](#29-broadcasting)).

### 2.8 Gas estimation

Use the `simulate` query to dry-run a transaction:

```graphql
query Simulate($tx: UnsignedTx!) {
  simulate(tx: $tx)
}
```

**Variables:**

```json
{
  "tx": {
    "sender": "0x1234...abcd",
    "msgs": [
      {
        "execute": {
          "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
          "msg": { "trade": { "deposit": {} } },
          "funds": { "usdc": "1000000000" }
        }
      }
    ],
    "data": {
      "user_index": 0,
      "chain_id": "dango-1",
      "nonce": 42,
      "expiry": null
    }
  }
}
```

**Response:**

```json
{
  "data": {
    "simulate": {
      "gas_limit": null,
      "gas_used": 750000,
      "result": { "ok": [ ... ] }
    }
  }
}
```

Simulation skips signature verification. Add **770,000 gas** (Secp256k1 verification cost) to `gas_used` when setting `gas_limit` in the final transaction.

### 2.9 Broadcasting

Submit a signed transaction:

```graphql
mutation BroadcastTx($tx: Tx!) {
  broadcastTxSync(tx: $tx)
}
```

**Variables:**

```json
{
  "tx": {
    "sender": "0x1234...abcd",
    "gas_limit": 1500000,
    "msgs": [ ... ],
    "data": { "user_index": 0, "chain_id": "dango-1", "nonce": 42, "expiry": null },
    "credential": { "standard": { "key_hash": "...", "signature": { ... } } }
  }
}
```

The mutation returns the transaction outcome as JSON.

## 3. Account management

Dango uses **smart accounts** instead of externally-owned accounts (EOAs). A user profile is identified by a `UserIndex` and may own multiple subaccounts (up to 5). Keys are associated with the user profile, not individual accounts.

### 3.1 Contract address discovery

Query the chain's app config to discover contract addresses:

```graphql
query {
  queryApp(request: { appConfig: {} })
}
```

**Response (abbreviated):**

```json
{
  "data": {
    "queryApp": {
      "addresses": {
        "account_factory": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
        "perps": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
        "oracle": "0xcedc5f73cbb963a48471b849c3650e6e34cd3b6d",
        "dex": "0xda32476efe31e535207f0ad690d337a4ebf54a22",
        "gateway": "0xc51e2cbe9636a90c86463ac3eb18fbee92b700d1",
        "taxman": "0xda70a9c1417aee00f960fe896add9d571f9c365b"
      },
      "minimum_deposit": [{ "denom": "usdc", "amount": "10000000" }],
      "maker_fee_rate": "0.002500",
      "taker_fee_rate": "0.004000"
    }
  }
}
```

| Address           | Contract                          |
| ----------------- | --------------------------------- |
| `account_factory` | User profiles, accounts, and keys |
| `perps`           | Perpetual futures exchange        |
| `oracle`          | Price oracle                      |
| `dex`             | Spot DEX                          |

### 3.2 Register user

Create a new user profile with an initial key and master account. This is a two-step process: first, send an initial deposit to the account factory; then call `register_user`.

```json
{
  "execute": {
    "contract": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
    "msg": {
      "register_user": {
        "key": { "secp256r1": "02abc123...33bytes_hex" },
        "key_hash": "a1b2c3d4...64hex",
        "seed": 12345,
        "signature": {
          "passkey": {
            "authenticator_data": "<base64>",
            "client_data": "<base64>",
            "sig": "0102...40hex"
          }
        }
      }
    },
    "funds": { "usdc": "10000000" }
  }
}
```

| Field       | Type        | Description                                                    |
| ----------- | ----------- | -------------------------------------------------------------- |
| `key`       | `Key`       | The user's initial public key (see [10.3](#103-enums))         |
| `key_hash`  | `Hash256`   | Client-chosen hash identifying this key                        |
| `seed`      | `u32`       | Arbitrary number for address variety                           |
| `signature` | `Signature` | Signature over `{"chain_id": "dango-1"}` proving key ownership |

The `funds` must meet the `minimum_deposit` from the app config. A master account is created automatically and returned via events.

### 3.3 Register subaccount

Create an additional account for an existing user (maximum 5 accounts per user):

```json
{
  "execute": {
    "contract": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
    "msg": { "register_account": {} },
    "funds": {}
  }
}
```

Must be sent from an existing account owned by the user.

### 3.4 Update key

Associate or disassociate a key with the user profile.

**Add a key:**

```json
{
  "execute": {
    "contract": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
    "msg": {
      "update_key": {
        "key_hash": "a1b2c3d4...64hex",
        "key": { "set": { "secp256k1": "03def456...33bytes_hex" } }
      }
    },
    "funds": {}
  }
}
```

**Remove a key:**

```json
{
  "execute": {
    "contract": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
    "msg": {
      "update_key": {
        "key_hash": "a1b2c3d4...64hex",
        "key": "unset"
      }
    },
    "funds": {}
  }
}
```

### 3.5 Update username

Set the user's human-readable username (one-time operation):

```json
{
  "execute": {
    "contract": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
    "msg": { "update_username": "alice" },
    "funds": {}
  }
}
```

Username rules: 1–15 characters, lowercase `a-z`, digits `0-9`, and underscore `_` only.

### 3.6 Query user (on-chain)

**By index:**

```graphql
query {
  queryApp(request: {
    wasmSmart: {
      contract: "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
      msg: { user: { index: 0 } }
    }
  })
}
```

**By username:**

```graphql
query {
  queryApp(request: {
    wasmSmart: {
      contract: "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
      msg: { user: { name: "alice" } }
    }
  })
}
```

**Response:**

```json
{
  "index": 0,
  "name": "alice",
  "accounts": {
    "0": "0x1234...abcd",
    "1": "0x5678...ef01"
  },
  "keys": {
    "a1b2c3d4...64hex": { "secp256r1": "02abc123...33bytes_hex" }
  }
}
```

### 3.7 Query users and accounts (on-chain)

**Enumerate users (paginated):**

```json
{ "users": { "start_after": null, "limit": 10 } }
```

Returns: `{ "<user_index>": <User>, ... }`

**Query account by address:**

```json
{ "account": { "address": "0x1234...abcd" } }
```

Returns: `{ "index": 0, "owner": 0 }`

**Enumerate accounts (paginated):**

```json
{ "accounts": { "start_after": null, "limit": 10 } }
```

Returns: `{ "<address>": { "index": <n>, "owner": <user_index> }, ... }`

**Find user by key hash** (useful if the user forgot their username):

```json
{ "forgot_username": { "key_hash": "a1b2c3d4...64hex", "start_after": null, "limit": 10 } }
```

Returns: `[<User>, ...]`

**Next user index** (useful for tracking registration count):

```json
{ "next_user_index": {} }
```

Returns: `u32` (e.g. `42`) — the index that will be assigned to the next registered user.

**Next account index:**

```json
{ "next_account_index": {} }
```

Returns: `u32` (e.g. `85`) — the index that will be assigned to the next created account.

### 3.8 Query user (indexer)

```graphql
query {
  user(userIndex: 0) {
    userIndex
    createdBlockHeight
    createdAt
    publicKeys {
      keyHash
      publicKey
      keyType
      createdBlockHeight
      createdAt
    }
    accounts {
      accountIndex
      address
      createdBlockHeight
      createdAt
    }
  }
}
```

The `keyType` enum values are: `SECP_25_6R_1`, `SECP_25_6K_1`, `ETHEREUM`.

### 3.9 Query accounts (indexer)

```graphql
query {
  accounts(userIndex: 0, first: 10) {
    nodes {
      accountIndex
      address
      createdBlockHeight
      createdTxHash
      createdAt
      users { userIndex }
    }
    pageInfo { hasNextPage endCursor }
  }
}
```

Filter by `userIndex` to get all accounts for a specific user, or by `address` for a specific account.

## 4. Market data

All on-chain queries in this section use the `queryApp` field targeting the perps contract at `0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69`.

### 4.1 Pair parameters

**All pairs:**

```graphql
query {
  queryApp(request: {
    wasmSmart: {
      contract: "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
      msg: { pair_params: { start_after: null, limit: 30 } }
    }
  })
}
```

**Single pair:**

```json
{ "pair_param": { "pair_id": "perp/btcusd" } }
```

**Response (single pair):**

```json
{
  "tick_size": "1.000000",
  "min_order_size": "10.000000",
  "max_abs_oi": "1000000.000000",
  "max_abs_funding_rate": "0.000500",
  "initial_margin_ratio": "0.050000",
  "maintenance_margin_ratio": "0.025000",
  "impact_size": "10000.000000",
  "vault_liquidity_weight": "1.000000",
  "vault_half_spread": "0.001000",
  "vault_max_quote_size": "50000.000000",
  "bucket_sizes": ["1.000000", "5.000000", "10.000000"]
}
```

| Field                      | Type            | Description                                   |
| -------------------------- | --------------- | --------------------------------------------- |
| `tick_size`                | `UsdPrice`      | Minimum price increment for limit orders      |
| `min_order_size`           | `UsdValue`      | Minimum notional value (reduce-only exempt)   |
| `max_abs_oi`               | `Quantity`      | Maximum open interest per side                |
| `max_abs_funding_rate`     | `FundingRate`   | Daily funding rate cap                        |
| `initial_margin_ratio`     | `Dimensionless` | Margin to open (e.g. 0.05 = 20x max leverage) |
| `maintenance_margin_ratio` | `Dimensionless` | Margin to stay open (liquidation threshold)   |
| `impact_size`              | `UsdValue`      | Notional for impact price calculation         |
| `vault_liquidity_weight`   | `Dimensionless` | Vault allocation weight for this pair         |
| `vault_half_spread`        | `Dimensionless` | Half the vault's bid-ask spread               |
| `vault_max_quote_size`     | `Quantity`      | Maximum vault resting size per side           |
| `bucket_sizes`             | `[UsdPrice]`    | Price bucket granularities for depth queries  |

For the relationship between margin ratios and leverage, see [Risk §2](6-risk.md).

### 4.2 Pair state

**All pairs:**

```json
{ "pair_states": { "start_after": null, "limit": 30 } }
```

**Single pair:**

```json
{ "pair_state": { "pair_id": "perp/btcusd" } }
```

**Response:**

```json
{
  "long_oi": "12500.000000",
  "short_oi": "10300.000000",
  "funding_per_unit": "0.000123"
}
```

| Field              | Type             | Description                    |
| ------------------ | ---------------- | ------------------------------ |
| `long_oi`          | `Quantity`       | Total long open interest       |
| `short_oi`         | `Quantity`       | Total short open interest      |
| `funding_per_unit` | `FundingPerUnit` | Cumulative funding accumulator |

For funding mechanics, see [Funding](3-funding.md).

### 4.3 Global state

```json
{ "state": {} }
```

**Response:**

```json
{
  "last_funding_time": "1700000000000000000",
  "vault_share_supply": "500000000",
  "insurance_fund": "25000.000000",
  "treasury": "12000.000000"
}
```

| Field                | Type        | Description                  |
| -------------------- | ----------- | ---------------------------- |
| `last_funding_time`  | `Timestamp` | Last funding collection time |
| `vault_share_supply` | `Uint128`   | Total vault share tokens     |
| `insurance_fund`     | `UsdValue`  | Insurance fund balance       |
| `treasury`           | `UsdValue`  | Accumulated protocol fees    |

### 4.4 Global parameters

```json
{ "param": {} }
```

**Response:**

```json
{
  "max_unlocks": 5,
  "max_open_orders": 50,
  "max_conditional_orders": 20,
  "base_maker_fee_rate": "0.000000",
  "base_taker_fee_rate": "0.001000",
  "tiered_maker_fee_rate": {},
  "tiered_taker_fee_rate": {},
  "protocol_fee_rate": "0.100000",
  "liquidation_fee_rate": "0.010000",
  "funding_period": "3600000000000",
  "vault_total_weight": "10.000000",
  "vault_cooldown_period": "604800000000000"
}
```

| Field                    | Type                           | Description                                                |
| ------------------------ | ------------------------------ | ---------------------------------------------------------- |
| `max_unlocks`            | `usize`                        | Max concurrent vault unlock requests per user              |
| `max_open_orders`        | `usize`                        | Max resting limit orders per user (all pairs)              |
| `max_conditional_orders` | `usize`                        | Max TP/SL orders per user (all pairs)                      |
| `base_maker_fee_rate`    | `Dimensionless`                | Maker fee when no volume tier qualifies                    |
| `base_taker_fee_rate`    | `Dimensionless`                | Taker fee when no volume tier qualifies                    |
| `tiered_maker_fee_rate`  | `Map<UsdValue, Dimensionless>` | Volume-tiered maker fees (threshold → rate)                |
| `tiered_taker_fee_rate`  | `Map<UsdValue, Dimensionless>` | Volume-tiered taker fees (threshold → rate)                |
| `protocol_fee_rate`      | `Dimensionless`                | Fraction of trading fees routed to treasury                |
| `liquidation_fee_rate`   | `Dimensionless`                | Insurance fund fee on liquidations                         |
| `funding_period`         | `Duration`                     | Interval between funding collections (nanoseconds)         |
| `vault_total_weight`     | `Dimensionless`                | Sum of all pairs' vault liquidity weights                  |
| `vault_cooldown_period`  | `Duration`                     | Waiting time before vault withdrawal release (nanoseconds) |

For fee mechanics, see [Order matching §8](2-order-matching.md#8-trading-fees).

### 4.5 Order book depth

Query aggregated order book depth at a given price bucket granularity:

```graphql
query {
  queryApp(request: {
    wasmSmart: {
      contract: "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
      msg: {
        liquidity_depth: {
          pair_id: "perp/btcusd",
          bucket_size: "10.000000",
          limit: 20
        }
      }
    }
  })
}
```

| Parameter     | Type       | Description                                               |
| ------------- | ---------- | --------------------------------------------------------- |
| `pair_id`     | `PairId`   | Trading pair                                              |
| `bucket_size` | `UsdPrice` | Price aggregation granularity (must be in `bucket_sizes`) |
| `limit`       | `u32?`     | Max number of price levels per side                       |

**Response:**

```json
{
  "bids": {
    "64990.000000": { "size": "12.500000", "notional": "812375.000000" },
    "64980.000000": { "size": "8.200000", "notional": "532836.000000" }
  },
  "asks": {
    "65010.000000": { "size": "10.000000", "notional": "650100.000000" },
    "65020.000000": { "size": "5.500000", "notional": "357610.000000" }
  }
}
```

Each level contains:

| Field      | Type       | Description                       |
| ---------- | ---------- | --------------------------------- |
| `size`     | `Quantity` | Absolute order size in the bucket |
| `notional` | `UsdValue` | USD notional (size × price)       |

### 4.6 24h statistics (indexer)

**All pairs:**

```graphql
query {
  allPairStats {
    baseDenom
    quoteDenom
    currentPrice
    price24HAgo
    volume24H
    priceChange24H
  }
}
```

**Single pair:**

```graphql
query {
  pairStats(baseDenom: "perp/btcusd", quoteDenom: "uusdc") {
    currentPrice
    price24HAgo
    volume24H
    priceChange24H
  }
}
```

| Field            | Type         | Description                                        |
| ---------------- | ------------ | -------------------------------------------------- |
| `currentPrice`   | `BigDecimal` | Current price (nullable)                           |
| `price24HAgo`    | `BigDecimal` | Price 24 hours ago (nullable)                      |
| `volume24H`      | `BigDecimal` | 24h trading volume in quote asset                  |
| `priceChange24H` | `BigDecimal` | 24h price change percentage (e.g. `5.25` = +5.25%) |

### 4.7 Historical candles (indexer)

```graphql
query {
  perpsCandles(
    pairId: "perp/btcusd",
    interval: ONE_HOUR,
    laterThan: "2026-01-01T00:00:00Z",
    earlierThan: "2026-01-02T00:00:00Z",
    first: 24
  ) {
    nodes {
      pairId
      interval
      open
      high
      low
      close
      volume
      volumeUsd
      timeStart
      timeStartUnix
      timeEnd
      timeEndUnix
      minBlockHeight
      maxBlockHeight
    }
    pageInfo { hasNextPage endCursor }
  }
}
```

| Parameter     | Type              | Description                          |
| ------------- | ----------------- | ------------------------------------ |
| `pairId`      | `String!`         | Trading pair (e.g. `"perp/btcusd"`)  |
| `interval`    | `CandleInterval!` | Candle interval                      |
| `laterThan`   | `DateTime`        | Candles after this time (inclusive)  |
| `earlierThan` | `DateTime`        | Candles before this time (exclusive) |

**CandleInterval values:** `ONE_SECOND`, `ONE_MINUTE`, `FIVE_MINUTES`, `FIFTEEN_MINUTES`, `ONE_HOUR`, `FOUR_HOURS`, `ONE_DAY`, `ONE_WEEK`.

**PerpsCandle fields:**

| Field            | Type         | Description                   |
| ---------------- | ------------ | ----------------------------- |
| `open`           | `BigDecimal` | Opening price                 |
| `high`           | `BigDecimal` | Highest price                 |
| `low`            | `BigDecimal` | Lowest price                  |
| `close`          | `BigDecimal` | Closing price                 |
| `volume`         | `BigDecimal` | Volume in base units          |
| `volumeUsd`      | `BigDecimal` | Volume in USD                 |
| `timeStart`      | `String`     | Period start (ISO 8601)       |
| `timeStartUnix`  | `Int`        | Period start (Unix timestamp) |
| `timeEnd`        | `String`     | Period end (ISO 8601)         |
| `timeEndUnix`    | `Int`        | Period end (Unix timestamp)   |
| `minBlockHeight` | `Int`        | First block in this candle    |
| `maxBlockHeight` | `Int`        | Last block in this candle     |

## 5. User state and orders

### 5.1 User state

```graphql
query {
  queryApp(request: {
    wasmSmart: {
      contract: "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
      msg: { user_state: { user: "0x1234...abcd" } }
    }
  })
}
```

**Response:**

```json
{
  "margin": "10000.000000",
  "vault_shares": "0",
  "positions": {
    "perp/btcusd": {
      "size": "0.500000",
      "entry_price": "64500.000000",
      "entry_funding_per_unit": "0.000100"
    }
  },
  "unlocks": [],
  "reserved_margin": "500.000000",
  "open_order_count": 2,
  "conditional_order_count": 1
}
```

| Field                     | Type                    | Description                              |
| ------------------------- | ----------------------- | ---------------------------------------- |
| `margin`                  | `UsdValue`              | Deposited margin (USD)                   |
| `vault_shares`            | `Uint128`               | Vault liquidity shares owned             |
| `positions`               | `Map<PairId, Position>` | Open positions by pair                   |
| `unlocks`                 | `[Unlock]`              | Pending vault withdrawals                |
| `reserved_margin`         | `UsdValue`              | Margin reserved for resting limit orders |
| `open_order_count`        | `usize`                 | Number of resting limit orders           |
| `conditional_order_count` | `usize`                 | Number of TP/SL orders                   |

**Position:**

| Field                    | Type             | Description                                       |
| ------------------------ | ---------------- | ------------------------------------------------- |
| `size`                   | `Quantity`       | Position size (positive = long, negative = short) |
| `entry_price`            | `UsdPrice`       | Average entry price                               |
| `entry_funding_per_unit` | `FundingPerUnit` | Funding accumulator at last modification          |

**Unlock:**

| Field               | Type        | Description             |
| ------------------- | ----------- | ----------------------- |
| `end_time`          | `Timestamp` | When cooldown completes |
| `amount_to_release` | `UsdValue`  | USD value to release    |

Returns `null` if the user has no state.

**Enumerate all user states (paginated):**

```json
{ "user_states": { "start_after": null, "limit": 10 } }
```

Returns: `{ "<address>": <UserState>, ... }`

### 5.2 Open orders

Query all resting limit orders and conditional (TP/SL) orders for a user:

```json
{ "orders_by_user": { "user": "0x1234...abcd" } }
```

**Response:**

```json
{
  "42": {
    "pair_id": "perp/btcusd",
    "size": "0.500000",
    "kind": {
      "limit": {
        "limit_price": "63000.000000",
        "reduce_only": false,
        "reserved_margin": "1575.000000"
      }
    },
    "created_at": "1700000000000000000"
  },
  "43": {
    "pair_id": "perp/btcusd",
    "size": "-0.500000",
    "kind": {
      "conditional": {
        "trigger_price": "70000.000000",
        "trigger_direction": "above"
      }
    },
    "created_at": "1700000100000000000"
  }
}
```

The response is a map of `OrderId` → order details. The `kind` field is either `limit` or `conditional`.

### 5.3 Single order

```json
{ "order": { "order_id": "42" } }
```

**Response:**

```json
{
  "user": "0x1234...abcd",
  "pair_id": "perp/btcusd",
  "size": "0.500000",
  "kind": {
    "limit": {
      "limit_price": "63000.000000",
      "reduce_only": false,
      "reserved_margin": "1575.000000"
    }
  },
  "created_at": "1700000000000000000"
}
```

Returns `null` if the order does not exist.

### 5.4 Trading volume

```json
{ "volume": { "user": "0x1234...abcd", "since": null } }
```

| Parameter | Type         | Description                                          |
| --------- | ------------ | ---------------------------------------------------- |
| `user`    | `Addr`       | Account address                                      |
| `since`   | `Timestamp?` | Start time (nanoseconds); `null` for lifetime volume |

Returns a `UsdValue` string (e.g. `"1250000.000000"`).

### 5.5 Trade history (indexer)

Query historical perps events such as fills, liquidations, and order lifecycle:

```graphql
query {
  perpsEvents(
    userAddr: "0x1234...abcd",
    eventType: "order_filled",
    pairId: "perp/btcusd",
    first: 50,
    sortBy: BLOCK_HEIGHT_DESC
  ) {
    nodes {
      idx
      blockHeight
      txHash
      eventType
      userAddr
      pairId
      data
      createdAt
    }
    pageInfo { hasNextPage endCursor }
  }
}
```

| Parameter     | Type     | Description                                          |
| ------------- | -------- | ---------------------------------------------------- |
| `userAddr`    | `String` | Filter by user address                               |
| `eventType`   | `String` | Filter by event type (see [§9](#9-events-reference)) |
| `pairId`      | `String` | Filter by trading pair                               |
| `blockHeight` | `Int`    | Filter by block height                               |

The `data` field contains the event-specific payload as JSON. For example, an `order_filled` event:

```json
{
  "order_id": "42",
  "pair_id": "perp/btcusd",
  "user": "0x1234...abcd",
  "fill_price": "65000.000000",
  "fill_size": "0.100000",
  "closing_size": "0.000000",
  "opening_size": "0.100000",
  "realized_pnl": "0.000000",
  "fee": "6.500000"
}
```

## 6. Trading operations

All execute messages in this section target the perps contract at `0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69`. Each message is wrapped in a `Tx` as described in [§2](#2-authentication-and-transactions).

### 6.1 Deposit margin

Deposit USDC into the trading margin account:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "deposit": {} } },
    "funds": { "usdc": "1000000000" }
  }
}
```

The deposited USDC is converted to USD at a fixed rate of $1 per unit and credited to `user_state.margin`. In this example, `1000000000` base units = 1,000 USDC = $1,000.

### 6.2 Withdraw margin

Withdraw USD from the trading margin account:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "withdraw": { "amount": "500.000000" } } },
    "funds": {}
  }
}
```

| Field    | Type       | Description            |
| -------- | ---------- | ---------------------- |
| `amount` | `UsdValue` | USD amount to withdraw |

The USD amount is converted to USDC at the current oracle price (floor-rounded) and transferred to the sender.

### 6.3 Submit market order

Buy or sell at the best available prices with a slippage tolerance:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "0.100000",
          "kind": { "market": { "max_slippage": "0.010000" } },
          "reduce_only": false
        }
      }
    },
    "funds": {}
  }
}
```

| Field          | Type            | Description                                                |
| -------------- | --------------- | ---------------------------------------------------------- |
| `pair_id`      | `PairId`        | Trading pair (e.g. `"perp/btcusd"`)                        |
| `size`         | `Quantity`      | Contract size — **positive = buy, negative = sell**        |
| `max_slippage` | `Dimensionless` | Maximum slippage as a fraction of oracle price (0.01 = 1%) |
| `reduce_only`  | `bool`          | If `true`, only the position-closing portion executes      |

Market orders execute immediately (IOC behavior). Any unfilled remainder is discarded. If nothing fills, the transaction reverts.

For order matching mechanics, see [Order matching](2-order-matching.md).

### 6.4 Submit limit order

Place a resting order on the book:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "-0.500000",
          "kind": {
            "limit": {
              "limit_price": "65000.000000",
              "post_only": false
            }
          },
          "reduce_only": false
        }
      }
    },
    "funds": {}
  }
}
```

| Field         | Type       | Description                                                    |
| ------------- | ---------- | -------------------------------------------------------------- |
| `limit_price` | `UsdPrice` | Limit price — must be aligned to `tick_size`                   |
| `post_only`   | `bool`     | If `true`, rejected if it would match immediately (maker-only) |
| `reduce_only` | `bool`     | If `true`, only position-closing portion is kept               |

Limit orders are GTC (good-till-cancelled). The matching portion fills immediately; any unfilled remainder is stored on the book. Margin is reserved for the unfilled portion.

### 6.5 Cancel order

**Cancel a single order:**

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "cancel_order": { "one": "42" } } },
    "funds": {}
  }
}
```

**Cancel all orders:**

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "cancel_order": "all" } },
    "funds": {}
  }
}
```

Cancellation releases reserved margin and decrements `open_order_count`.

### 6.6 Submit conditional order (TP/SL)

Place a take-profit or stop-loss order that triggers when the oracle price crosses a threshold:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "trade": {
        "submit_conditional_order": {
          "pair_id": "perp/btcusd",
          "size": "-0.100000",
          "trigger_price": "70000.000000",
          "trigger_direction": "above",
          "max_slippage": "0.020000"
        }
      }
    },
    "funds": {}
  }
}
```

| Field               | Type               | Description                                        |
| ------------------- | ------------------ | -------------------------------------------------- |
| `pair_id`           | `PairId`           | Trading pair                                       |
| `size`              | `Quantity`         | Size to close — sign must oppose the position      |
| `trigger_price`     | `UsdPrice`         | Oracle price that activates this order             |
| `trigger_direction` | `TriggerDirection` | `"above"` or `"below"` (see below)                 |
| `max_slippage`      | `Dimensionless`    | Slippage tolerance for the market order at trigger |

**Trigger direction:**

| Direction | Triggers when                 | Use case                                    |
| --------- | ----------------------------- | ------------------------------------------- |
| `above`   | oracle_price >= trigger_price | Take-profit for longs, stop-loss for shorts |
| `below`   | oracle_price <= trigger_price | Stop-loss for longs, take-profit for shorts |

Conditional orders are always **reduce-only** with zero reserved margin. When triggered, they execute as market orders.

### 6.7 Cancel conditional order

**Cancel a single conditional order:**

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "cancel_conditional_order": { "one": "43" } } },
    "funds": {}
  }
}
```

**Cancel all conditional orders:**

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "trade": { "cancel_conditional_order": "all" } },
    "funds": {}
  }
}
```

### 6.8 Liquidate (permissionless)

Force-close all positions of an undercollateralized user. This message can be sent by anyone (liquidation bots):

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": { "maintain": { "liquidate": { "user": "0x5678...ef01" } } },
    "funds": {}
  }
}
```

The transaction reverts if the user is not below the maintenance margin. Unfilled positions are ADL'd against counter-parties at the bankruptcy price. For mechanics, see [Liquidation & ADL](4-liquidation-and-adl.md).

## 7. Vault operations

The counterparty vault provides liquidity for the exchange. Users can deposit margin into the vault to earn trading fees, and withdraw with a cooldown period.

### 7.1 Add liquidity

Transfer margin from the trading account to the vault:

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "vault": {
        "add_liquidity": {
          "amount": "1000.000000",
          "min_shares_to_mint": "900000"
        }
      }
    },
    "funds": {}
  }
}
```

| Field                | Type       | Description                                        |
| -------------------- | ---------- | -------------------------------------------------- |
| `amount`             | `UsdValue` | USD margin amount to transfer to the vault         |
| `min_shares_to_mint` | `Uint128?` | Revert if fewer shares are minted (slippage guard) |

Shares are minted proportionally to the vault's current NAV. For vault mechanics, see [Vault](5-vault.md).

### 7.2 Remove liquidity

Request a withdrawal from the vault (initiates cooldown):

```json
{
  "execute": {
    "contract": "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
    "msg": {
      "vault": {
        "remove_liquidity": {
          "shares_to_burn": "500000"
        }
      }
    },
    "funds": {}
  }
}
```

| Field            | Type      | Description              |
| ---------------- | --------- | ------------------------ |
| `shares_to_burn` | `Uint128` | Number of shares to burn |

Shares are burned immediately. The corresponding USD value enters a cooldown queue. After `vault_cooldown_period` elapses, funds are automatically credited back to the user's trading margin.

## 8. Real-time subscriptions

All subscriptions use the WebSocket transport described in [§1.2](#12-websocket).

### 8.1 Perps candles

Stream OHLCV candlestick data for a perpetual pair:

```graphql
subscription {
  perpsCandles(pairId: "perp/btcusd", interval: ONE_MINUTE) {
    pairId
    interval
    open
    high
    low
    close
    volume
    volumeUsd
    timeStart
    timeStartUnix
    timeEnd
    timeEndUnix
    minBlockHeight
    maxBlockHeight
  }
}
```

Pushes updated candle data as new trades occur. Fields match the `PerpsCandle` type in [§4.7](#47-historical-candles-indexer).

### 8.2 Perps trades

Stream real-time trade fills for a pair:

```graphql
subscription {
  perpsTrades(pairId: "perp/btcusd") {
    orderId
    pairId
    user
    fillPrice
    fillSize
    closingSize
    openingSize
    realizedPnl
    fee
    createdAt
    blockHeight
    tradeIdx
  }
}
```

**Behavior:** On connection, cached recent trades are replayed first, then new trades stream in real-time.

| Field         | Type     | Description                                   |
| ------------- | -------- | --------------------------------------------- |
| `orderId`     | `String` | Order ID that produced this fill              |
| `pairId`      | `String` | Trading pair                                  |
| `user`        | `String` | Account address                               |
| `fillPrice`   | `String` | Execution price                               |
| `fillSize`    | `String` | Filled size (positive = buy, negative = sell) |
| `closingSize` | `String` | Portion that closed existing position         |
| `openingSize` | `String` | Portion that opened new position              |
| `realizedPnl` | `String` | PnL realized from the closing portion         |
| `fee`         | `String` | Trading fee charged                           |
| `createdAt`   | `String` | Timestamp (ISO 8601)                          |
| `blockHeight` | `Int`    | Block in which the trade occurred             |
| `tradeIdx`    | `Int`    | Index within the block                        |

### 8.3 Contract query polling

Poll any on-chain contract query at a regular block interval:

```graphql
subscription {
  queryApp(
    request: {
      wasmSmart: {
        contract: "0xd04b99adca5d3d31a1e7bc72fd606202f1e2fc69",
        msg: { user_state: { user: "0x1234...abcd" } }
      }
    },
    blockInterval: 5
  ) {
    response
    blockHeight
  }
}
```

| Parameter       | Type             | Default | Description                  |
| --------------- | ---------------- | ------- | ---------------------------- |
| `request`       | `GrugQueryInput` | —       | Any valid `queryApp` request |
| `blockInterval` | `Int`            | `10`    | Push updates every N blocks  |

Common use cases:

- **User state** — monitor margin, positions, and order counts.
- **Order book depth** — track bid/ask levels.
- **Pair states** — monitor open interest and funding.

### 8.4 Block stream

Subscribe to new blocks as they are finalized:

```graphql
subscription {
  block {
    blockHeight
    hash
    appHash
    createdAt
  }
}
```

### 8.5 Event stream

Subscribe to events with optional filtering:

```graphql
subscription {
  events(
    sinceBlockHeight: 100000,
    filter: [
      {
        type: "order_filled",
        data: [
          { path: ["user"], checkMode: EQUAL, value: ["0x1234...abcd"] }
        ]
      }
    ]
  ) {
    type
    method
    eventStatus
    data
    blockHeight
    createdAt
  }
}
```

| Filter field | Type           | Description                                     |
| ------------ | -------------- | ----------------------------------------------- |
| `type`       | `String`       | Event type name                                 |
| `data`       | `[FilterData]` | Conditions on the event's JSON data             |
| `path`       | `[String]`     | JSON path to the field                          |
| `checkMode`  | `CheckValue`   | `EQUAL` (exact match) or `CONTAINS` (substring) |
| `value`      | `[JSON]`       | Values to match against                         |

## 9. Events reference

The perps contract emits the following events. These can be queried via `perpsEvents` ([§5.5](#55-trade-history-indexer)) or streamed via the `events` subscription ([§8.5](#85-event-stream)).

### Margin events

| Event       | Fields           | Description      |
| ----------- | ---------------- | ---------------- |
| `deposited` | `user`, `amount` | Margin deposited |
| `withdrew`  | `user`, `amount` | Margin withdrawn |

### Vault events

| Event                 | Fields                                        | Description                        |
| --------------------- | --------------------------------------------- | ---------------------------------- |
| `liquidity_added`     | `user`, `amount`, `shares_minted`             | LP deposited to vault              |
| `liquidity_unlocking` | `user`, `amount`, `shares_burned`, `end_time` | LP withdrawal initiated (cooldown) |
| `liquidity_released`  | `user`, `amount`                              | Cooldown completed, funds released |

### Order events

| Event             | Fields                                                                                                          | Description                     |
| ----------------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `order_filled`    | `order_id`, `pair_id`, `user`, `fill_price`, `fill_size`, `closing_size`, `opening_size`, `realized_pnl`, `fee` | Order partially or fully filled |
| `order_persisted` | `order_id`, `pair_id`, `user`, `limit_price`, `size`                                                            | Limit order placed on book      |
| `order_removed`   | `order_id`, `pair_id`, `user`, `reason`                                                                         | Order removed from book         |

### Conditional order events

| Event                         | Fields                                                                                      | Description                   |
| ----------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------- |
| `conditional_order_placed`    | `order_id`, `pair_id`, `user`, `trigger_price`, `trigger_direction`, `size`, `max_slippage` | TP/SL order created           |
| `conditional_order_triggered` | `order_id`, `pair_id`, `user`, `trigger_price`, `oracle_price`                              | TP/SL triggered by price move |
| `conditional_order_removed`   | `order_id`, `pair_id`, `user`, `reason`                                                     | TP/SL removed                 |

### Liquidation events

| Event              | Fields                                                          | Description                      |
| ------------------ | --------------------------------------------------------------- | -------------------------------- |
| `liquidated`       | `user`, `pair_id`, `adl_size`, `adl_price`                      | Position liquidated in a pair    |
| `deleveraged`      | `user`, `pair_id`, `closing_size`, `fill_price`, `realized_pnl` | Counter-party hit by ADL         |
| `bad_debt_covered` | `liquidated_user`, `amount`, `insurance_fund_remaining`         | Insurance fund absorbed bad debt |

### ReasonForOrderRemoval

| Value                   | Description                                                    |
| ----------------------- | -------------------------------------------------------------- |
| `filled`                | Order fully filled                                             |
| `canceled`              | User voluntarily canceled                                      |
| `position_closed`       | Position was closed (conditional orders only)                  |
| `self_trade_prevention` | Order crossed user's own order on the opposite side            |
| `liquidated`            | User was liquidated                                            |
| `deleveraged`           | User was hit by auto-deleveraging                              |
| `slippage_exceeded`     | Conditional order triggered but could not fill within slippage |

For liquidation and ADL mechanics, see [Liquidation & ADL](4-liquidation-and-adl.md).

## 10. Types reference

### 10.1 Numeric types

All numeric types are **signed fixed-point decimals with 6 decimal places** (`Dec128_6`), serialized as strings:

| Type alias       | Dimension      | Example usage                          | Example value    |
| ---------------- | -------------- | -------------------------------------- | ---------------- |
| `Dimensionless`  | (pure scalar)  | Fee rates, margin ratios, slippage     | `"0.050000"`     |
| `Quantity`       | quantity       | Position size, order size, OI          | `"-0.500000"`    |
| `UsdValue`       | usd            | Margin, PnL, notional, fees            | `"10000.000000"` |
| `UsdPrice`       | usd / quantity | Oracle price, limit price, entry price | `"65000.000000"` |
| `FundingPerUnit` | usd / quantity | Cumulative funding accumulator         | `"0.000123"`     |
| `FundingRate`    | per day        | Funding rate cap                       | `"0.000500"`     |

Additional integer types:

| Type      | Encoding         | Description                       |
| --------- | ---------------- | --------------------------------- |
| `Uint128` | String           | Large integer (e.g. vault shares) |
| `u64`     | Number or String | Gas limit, timestamps             |
| `u32`     | Number           | User index, account index, nonce  |

### 10.2 Identifiers

| Type                 | Format                          | Example                          |
| -------------------- | ------------------------------- | -------------------------------- |
| `PairId`             | `perp/<base><quote>`            | `"perp/btcusd"`, `"perp/ethusd"` |
| `OrderId`            | `Uint64` (string)               | `"42"`                           |
| `ConditionalOrderId` | `Uint64` (shared counter)       | `"43"`                           |
| `Addr`               | Hex address                     | `"0x1234...abcd"`                |
| `Hash256`            | 64-char hex                     | `"a1b2c3d4e5f6..."`              |
| `UserIndex`          | `u32`                           | `0`                              |
| `AccountIndex`       | `u32`                           | `1`                              |
| `Username`           | 1–15 chars, `[a-z0-9_]`         | `"alice"`                        |
| `Timestamp`          | Nanoseconds since epoch (`u64`) | `"1700000000000000000"`          |
| `Duration`           | Nanoseconds (`u64`)             | `"3600000000000"` (1 hour)       |

### 10.3 Enums

**OrderKind:**

```json
{ "market": { "max_slippage": "0.010000" } }
```

```json
{ "limit": { "limit_price": "65000.000000", "post_only": false } }
```

**TriggerDirection:**

```json
"above"
"below"
```

**CancelOrderRequest:**

```json
{ "one": "42" }
```

```json
"all"
```

**Key:**

```json
{ "secp256r1": "02abc123...33bytes_hex" }
{ "secp256k1": "03def456...33bytes_hex" }
{ "ethereum": "0x1234...abcd" }
```

**Credential:**

```json
{ "standard": { "key_hash": "...", "signature": { ... } } }
{ "session": { "session_info": { ... }, "session_signature": "...", "authorization": { ... } } }
```

**CandleInterval** (GraphQL enum):

`ONE_SECOND` | `ONE_MINUTE` | `FIVE_MINUTES` | `FIFTEEN_MINUTES` | `ONE_HOUR` | `FOUR_HOURS` | `ONE_DAY` | `ONE_WEEK`

### 10.4 Response types

**Param** (global parameters) — see [§4.4](#44-global-parameters) for all fields.

**PairParam** (per-pair parameters) — see [§4.1](#41-pair-parameters) for all fields.

**PairState:**

| Field              | Type             | Description                    |
| ------------------ | ---------------- | ------------------------------ |
| `long_oi`          | `Quantity`       | Total long open interest       |
| `short_oi`         | `Quantity`       | Total short open interest      |
| `funding_per_unit` | `FundingPerUnit` | Cumulative funding accumulator |

**State** (global state) — see [§4.3](#43-global-state) for all fields.

**UserState** — see [§5.1](#51-user-state) for all fields.

**Position:**

| Field                    | Type             | Description                        |
| ------------------------ | ---------------- | ---------------------------------- |
| `size`                   | `Quantity`       | Positive = long, negative = short  |
| `entry_price`            | `UsdPrice`       | Average entry price                |
| `entry_funding_per_unit` | `FundingPerUnit` | Funding accumulator at last update |

**Unlock:**

| Field               | Type        | Description             |
| ------------------- | ----------- | ----------------------- |
| `end_time`          | `Timestamp` | When cooldown completes |
| `amount_to_release` | `UsdValue`  | USD value to release    |

**QueryOrderResponse:**

| Field        | Type                      | Description               |
| ------------ | ------------------------- | ------------------------- |
| `user`       | `Addr`                    | Order owner               |
| `pair_id`    | `PairId`                  | Trading pair              |
| `size`       | `Quantity`                | Order size                |
| `kind`       | `LimitOrConditionalOrder` | Order type and parameters |
| `created_at` | `Timestamp`               | Creation time             |

**LimitOrConditionalOrder:**

```json
{ "limit": { "limit_price": "65000.000000", "reduce_only": false, "reserved_margin": "1575.000000" } }
```

```json
{ "conditional": { "trigger_price": "70000.000000", "trigger_direction": "above" } }
```

**LiquidityDepthResponse:**

| Field  | Type                            | Description             |
| ------ | ------------------------------- | ----------------------- |
| `bids` | `Map<UsdPrice, LiquidityDepth>` | Bid-side depth by price |
| `asks` | `Map<UsdPrice, LiquidityDepth>` | Ask-side depth by price |

**LiquidityDepth:**

| Field      | Type       | Description                   |
| ---------- | ---------- | ----------------------------- |
| `size`     | `Quantity` | Absolute order size in bucket |
| `notional` | `UsdValue` | USD notional (size × price)   |

**User** (account factory):

| Field      | Type                      | Description                      |
| ---------- | ------------------------- | -------------------------------- |
| `index`    | `UserIndex`               | User's numerical index           |
| `name`     | `Username`                | User's username                  |
| `accounts` | `Map<AccountIndex, Addr>` | Accounts owned (index → address) |
| `keys`     | `Map<Hash256, Key>`       | Associated keys (hash → key)     |

**Account:**

| Field   | Type           | Description            |
| ------- | -------------- | ---------------------- |
| `index` | `AccountIndex` | Account's unique index |
| `owner` | `UserIndex`    | Owning user's index    |
