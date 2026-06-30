# API Reference

This chapter documents the complete API for the Dango perpetual futures exchange. All interactions with the chain go through a single **GraphQL endpoint** that supports queries, mutations, and WebSocket subscriptions.

## 1. Transport

### 1.1 HTTP

All queries and mutations use a standard GraphQL POST request.

**Endpoint:** See [Constants](9-constants.md#endpoints).

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

### 1.2 WebSocket

Subscriptions (real-time data) use WebSocket with the `graphql-ws` protocol. A separate native WebSocket API (not `graphql-ws`) serves the `fullBlock` and `perpsEvents` feeds — see [§12](#12-new-in-v0260-websocket-api).

**Endpoint:** See [Constants](9-constants.md#endpoints).

**Connection handshake:**

```json
{
  "type": "connection_init",
  "payload": {}
}
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
{
  "id": "1",
  "type": "next",
  "payload": {
    "data": {
      "perpsTrades": { ... }
    }
  }
}
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

### 1.4 Multi-query

When you need to fetch multiple pieces of state (e.g. oracle prices and user positions), use the **multi-query** to execute them **atomically within a single block**. This is the preferred method over issuing separate GraphQL requests, which may be evaluated at different block heights and return an inconsistent snapshot.

Wrap the individual queries in a `multi` array:

```graphql
query {
  queryApp(request: {
    multi: [
      {
        wasm_smart: {
          contract: "ORACLE_CONTRACT",
          msg: { prices: {} }
        }
      },
      {
        wasm_smart: {
          contract: "PERPS_CONTRACT",
          msg: {
            user_state: { user: "0x1234...abcd" }
          }
        }
      }
    ]
  })
}
```

**Response:**

```json
{
  "multi": [
    { "Ok": { "wasm_smart": { /* oracle prices */ } } },
    { "Ok": { "wasm_smart": { /* user state */ } } }
  ]
}
```

Each element in the response array corresponds to the query at the same index in the request. Individual queries that fail return `{"Err": "..."}` without aborting the others.

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
        "contract": "PERPS_CONTRACT",
        "msg": { ... },
        "funds": {}
      }
    }
  ],
  "data": { ... },
  "credential": { ... }
}
```

| Field        | Type         | Description                                        |
| ------------ | ------------ | -------------------------------------------------- |
| `sender`     | `Addr`       | Account address sending the transaction            |
| `gas_limit`  | `u64`        | Maximum gas units for execution                    |
| `msgs`       | `[Message]`  | Non-empty list of messages to execute atomically   |
| `data`       | `Metadata`   | Authentication metadata (see [§2.2](#22-metadata)) |
| `credential` | `Credential` | Cryptographic proof of sender authorization        |

Messages execute **atomically** — either all succeed or all fail.

### 2.2 Metadata

The `data` field contains authentication metadata:

```json
{
  "user_index": 0,
  "chain_id": "dango-1",
  "nonce": 42,
  "expiry": null
}
```

| Field        | Type                | Description                                    |
| ------------ | ------------------- | ---------------------------------------------- |
| `user_index` | `u32`               | The user index that owns the sender account    |
| `chain_id`   | `String`            | Chain identifier (prevents cross-chain replay) |
| `nonce`      | `u32`               | Replay protection nonce                        |
| `expiry`     | `Timestamp \| null` | Optional expiration; `null` = no expiry        |

**Nonce semantics.** Dango uses **unordered nonces** with a sliding window, similar to [the approach used by Hyperliquid](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/nonces-and-api-wallets#hyperliquid-nonces). Crucially, nonces are tracked **per signer**, in two separate namespaces:

- A **standard credential** (signed with a master key) draws from a single account-wide window.
- A **session credential** draws from its own window, keyed by the session public key. This lets several clients — for example, one bot per session key — drive the same account concurrently without their nonces colliding.

Within either window, the account keeps the 20 most recently seen nonces. A transaction is accepted if its nonce has not been used before, is newer than the oldest nonce in the window, and is no greater than the newest seen nonce plus 100. Transactions may therefore arrive out of order without being rejected. Windows are never pruned.

The _first_ nonce accepted into an empty window is bounded:

- For a standard window — or a session window on an account that has never sent a transaction — the first nonce must be less than 100.
- For a session window on an account that _has_ transacted, the first nonce must be **greater than the account's standard-nonce high-water mark** (the largest nonce in the standard window). This rejects replays of session transactions that were signed before per-session windows existed. A client that picks `max + 1` always satisfies it.

SDK implementations should choose the next nonce client-side by querying the relevant window. Both queries target the **account's own contract** (the `sender` address) and return the seen nonces as an ascending array of integers.

A standard signer queries:

```json
{ "seen_nonces": {} }
```

and uses `max + 1` (or `0` if the array is empty).

A session signer queries (where `session_key` is the base64-encoded 33-byte compressed public key):

```json
{ "session_seen_nonces": { "session_key": "<base64>" } }
```

and uses `max + 1` of that array. If the session window is empty, it falls back to the standard window's `max + 1` (the floor above), or `0` if the account has never transacted.

### 2.3 Message format

The primary message type for interacting with contracts is `execute`:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "0.100000",
          "kind": {
            "market": {
              "max_slippage": "0.010000"
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

| Field      | Type    | Description                                                                |
| ---------- | ------- | -------------------------------------------------------------------------- |
| `contract` | `Addr`  | Target contract address                                                    |
| `msg`      | `JSON`  | Contract-specific execute message (snake_case keys)                        |
| `funds`    | `Coins` | Tokens to send with the message: `{"<denom>": "<amount>"}` or `{}` if none |

The `funds` field is a map of denomination to amount string. For example, depositing 1000 USDC:

```json
{
  "funds": {
    "bridge/usdc": "1000000000"
  }
}
```

USDC uses **6 decimal places** in its base unit (1 USDC = `1000000` base units). All bridged tokens use the `bridge/` prefix.

### 2.4 Signing methods

The `credential` field wraps a `StandardCredential` or `SessionCredential`. A `StandardCredential` identifies the signing key and contains the signature:

**Passkey (Secp256r1 / WebAuthn):**

```json
{
  "standard": {
    "key_hash": "A1B2C3D4...64HEX",
    "signature": {
      "passkey": {
        "authenticator_data": "<base64>",
        "client_data": "<base64>",
        "sig": "<base64>"
      }
    }
  }
}
```

- `sig`: 64-byte Secp256r1 signature (base64-encoded)
- `client_data`: base64-encoded WebAuthn client data JSON (challenge = base64url of SHA-256 of SignDoc)
- `authenticator_data`: base64-encoded WebAuthn authenticator data

**Secp256k1:**

```json
{
  "standard": {
    "key_hash": "A1B2C3D4...64HEX",
    "signature": {
      "secp256k1": "<base64>"
    }
  }
}
```

- 64-byte Secp256k1 signature (base64-encoded)

**EIP-712 (Ethereum wallets):**

```json
{
  "standard": {
    "key_hash": "A1B2C3D4...64HEX",
    "signature": {
      "eip712": {
        "typed_data": "<base64>",
        "sig": "<base64>"
      }
    }
  }
}
```

- `sig`: 65-byte signature (64-byte Secp256k1 + 1-byte recovery ID; base64-encoded)
- `typed_data`: base64-encoded JSON of the EIP-712 typed data object

### 2.5 Session credentials

Session keys allow delegated signing without requiring the master key for every transaction.

```json
{
  "session": {
    "session_info": {
      "session_key": "<base64>",
      "expire_at": "1700000000"
    },
    "session_signature": "<base64>",
    "authorization": {
      "key_hash": "A1B2C3D4...64HEX",
      "signature": { ... }
    }
  }
}
```

| Field               | Type                 | Description                                        |
| ------------------- | -------------------- | -------------------------------------------------- |
| `session_info`      | `SessionInfo`        | Session key public key + expiration                |
| `session_signature` | `ByteArray<64>`      | SignDoc signed by the session key (base64-encoded) |
| `authorization`     | `StandardCredential` | SessionInfo signed by the user's master key        |

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
2. **Fetch metadata** — query chain ID, the account's user_index, and the next available nonce. Standard signers read the nonce from the account-wide window; session signers read it from their per-session-key window (see [§2.2](#22-metadata)).
3. **Simulate** — send an `UnsignedTx` to estimate gas (see [§2.8](#28-gas-estimation)).
4. **Set gas limit** — use the simulation result, adding ~770,000 for signature verification overhead.
5. **Build SignDoc** — assemble `{sender, gas_limit, messages, data}`.
6. **Sign** — sign the SignDoc with the chosen method.
7. **Broadcast** — submit the signed `Tx` via `broadcastTxSync` (see [§2.9](#29-broadcasting)).

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
          "contract": "PERPS_CONTRACT",
          "msg": {
            "trade": {
              "deposit": {}
            }
          },
          "funds": {
            "bridge/usdc": "1000000000"
          }
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
      "result": {
        "ok": [ ... ]
      }
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
    "data": {
      "user_index": 0,
      "chain_id": "dango-1",
      "nonce": 42,
      "expiry": null
    },
    "credential": {
      "standard": {
        "key_hash": "...",
        "signature": { ... }
      }
    }
  }
}
```

The mutation returns the transaction outcome as JSON.

## 3. Account management

Dango uses **smart accounts** instead of externally-owned accounts (EOAs). A user profile is identified by a `UserIndex` and may own 1 master account and 0-4 subaccounts. Keys are associated with the user profile, not individual accounts.

### 3.1 Register user

Creating a new user profile is a two-step process:

**Step 1 — Register.** Call `register_user` on the account factory: use the **the account factory address itself as sender**, and `null` for the `data` and `credential` fields.

```json
{
  "sender": "ACCOUNT_FACTORY_CONTRACT",
  "gas_limit": 1500000,
  "msgs": [
    {
      "execute": {
        "contract": "ACCOUNT_FACTORY_CONTRACT",
        "msg": {
          "register_user": {
            "key": {
              "secp256r1": "<base64>"
            },
            "key_hash": "A1B2C3D4...64HEX",
            "seed": 12345,
            "signature": {
              "passkey": {
                "authenticator_data": "<base64>",
                "client_data": "<base64>",
                "sig": "<base64>"
              }
            }
          }
        },
        "funds": {}
      }
    }
  ],
  "data": null,
  "credential": null
}
```

| Field       | Type        | Description                                                    |
| ----------- | ----------- | -------------------------------------------------------------- |
| `key`       | `Key`       | The user's initial public key (see [§10.3](#103-enums))        |
| `key_hash`  | `Hash256`   | Client-chosen hash identifying this key                        |
| `seed`      | `u32`       | Arbitrary number for address variety                           |
| `signature` | `Signature` | Signature over `{"chain_id": "dango-1"}` proving key ownership |

A master account is created in the **inactive** state (for the purpose of spam prevention). The new account address is returned in the transaction events.

**Step 2 — Activate.** Send at least the `minimum_deposit` (10 USDC = `10000000` `bridge/usdc` on mainnet) to the new master account address. The transfer can either come from an existing Dango account, or from another chain via Hyperlane bridging. Upon receipt, the account activates itself and becomes ready to use. To programmatically confirm that a bridged transfer has arrived, see [§3.10](#310-query-bridge-deposit-delivery).

#### 3.1.1 Funding a new account via the faucet (testnet)

On testnet there is nothing of value to bridge, so in place of Step 2 you can call the public **faucet** to mint test tokens directly to the new master account. The account activates as soon as it receives the tokens. _The faucet exists on testnet only; there is no faucet on mainnet._

**Endpoint:** `GET https://faucet-testnet.dango.zone/mint/{address}` (see [Constants](9-constants.md#endpoints)).

A freshly registered account is empty and owns exactly one account, so it passes the faucet's eligibility checks and the bare call succeeds:

```bash
curl 'https://faucet-testnet.dango.zone/mint/0xYOUR_ACCOUNT_ADDRESS'
```

| Path parameter | Type   | Description                                                     |
| -------------- | ------ | --------------------------------------------------------------- |
| `address`      | `Addr` | The new master account's address, hex-encoded with `0x` prefix. |

| Query parameter | Type   | Default | Description                                                                                                                                                                                                                                                      |
| --------------- | ------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `skip_check`    | `bool` | `false` | When `false`, the faucet mints only to a brand-new account: it refuses if the address already holds any balance, or if the owning user has more than one account. Set to `true` to bypass both checks and mint unconditionally. The web app sets this to `true`. |

A successful call mints the following test tokens to the address (current amounts):

| Token   | Denom         | Amount  | Decimals |
| ------- | ------------- | ------- | -------- |
| USDC    | `bridge/usdc` | 200,000 | 6        |
| Ether   | `bridge/eth`  | 50      | 18       |
| Bitcoin | `bridge/btc`  | 2       | 8        |
| Solana  | `bridge/sol`  | 1,040   | 9        |
| XRP     | `bridge/xrp`  | 61,500  | 6        |

**Response.** On success, returns `200 OK` with the resulting transaction as JSON (hash, block height, events, outcome). On failure, returns `400 Bad Request` with a JSON error body, e.g.:

```json
{
  "error": "Tokens already minted",
  "address": "0x..."
}
```

The eligibility errors (only when `skip_check` is not set) are `"Tokens already minted"` and `"User already have multiple accounts"`. Transaction-level failures return `"Broadcast failed!"`, `"Tx failed"`, or `"Tx broadcasted but not found"` with the broadcast outcome under `data`.

A `GET /health` endpoint returns `{ "version": "<faucet version>" }` and can be used to check that the faucet is reachable.

### 3.2 Register subaccount

Create an additional account for an existing user (maximum 5 accounts per user):

```json
{
  "execute": {
    "contract": "ACCOUNT_FACTORY_CONTRACT",
    "msg": {
      "register_account": {}
    },
    "funds": {}
  }
}
```

Must be sent from an existing account owned by the user.

### 3.3 Account address derivation

#### 3.3.1 Master account

The first account of a new user ([§3.1](#31-register-user)) is derived as:

```plain
address := ripemd160(sha256(deployer || code_hash || seed || key_hash || key_tag || key))
```

where `||` denotes byte concatenation.

The preimage layout (122 bytes total):

| Byte range  | Size | Field       | Description                                                                                                   |
| ----------- | ---- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `[0..20)`   | 20   | `deployer`  | The `ACCOUNT_FACTORY_CONTRACT` address (see [Constants](9-constants.md#dango-contract-addresses))             |
| `[20..52)`  | 32   | `code_hash` | The code hash of the Dango single-signature account contract (see [Constants](9-constants.md#code-hashes))    |
| `[52..56)`  | 4    | `seed`      | User-chosen `u32`, big-endian — arbitrary value for frontrunning protection                                   |
| `[56..88)`  | 32   | `key_hash`  | Client-chosen 32-byte identifier for the key (see [§3.8](#38-query-users-by-key) for hashing rules)           |
| `[88..89)`  | 1    | `key_tag`   | Key type: `0` = Secp256r1, `1` = Secp256k1, `2` = Ethereum                                                    |
| `[89..122)` | 33   | `key`       | Secp256r1 / Secp256k1: 33-byte compressed public key. Ethereum: 13 zero bytes followed by the 20-byte address |

#### 3.3.2 Subaccount

```plain
address := ripemd160(sha256(deployer || code_hash || account_index))
```

The preimage layout (56 bytes total):

| Byte range | Size | Field           | Description                                                                                                |
| ---------- | ---- | --------------- | ---------------------------------------------------------------------------------------------------------- |
| `[0..20)`  | 20   | `deployer`      | The `ACCOUNT_FACTORY_CONTRACT` address (see [Constants](9-constants.md#dango-contract-addresses))          |
| `[20..52)` | 32   | `code_hash`     | The code hash of the Dango single-signature account contract (see [Constants](9-constants.md#code-hashes)) |
| `[52..56)` | 4    | `account_index` | Global account index, `u32`, big-endian                                                                    |

The global account index is a chain-wide monotonic counter maintained by the account factory; it is incremented for every account created across all users, so every account has a unique index.

### 3.4 Update key

Associate or disassociate a key with the user profile.

**Add a key:**

```json
{
  "execute": {
    "contract": "ACCOUNT_FACTORY_CONTRACT",
    "msg": {
      "update_key": {
        "key_hash": "A1B2C3D4...64HEX",
        "key": {
          "insert": {
            "secp256k1": "<base64>"
          }
        }
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
    "contract": "ACCOUNT_FACTORY_CONTRACT",
    "msg": {
      "update_key": {
        "key_hash": "A1B2C3D4...64HEX",
        "key": "delete"
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
    "contract": "ACCOUNT_FACTORY_CONTRACT",
    "msg": {
      "update_username": "alice"
    },
    "funds": {}
  }
}
```

Username rules: 1–15 characters, lowercase `a-z`, digits `0-9`, and underscore `_` only.

The username is cosmetic only — used for human-readable display on the frontend. It is not used in any business logic of the exchange.

### 3.6 Query user

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

The `keyType` enum values are: `SECP256R1`, `SECP256K1`, `ETHEREUM`.

### 3.7 Query user by username

Look up a user by their username via a smart contract query against the account factory:

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "ACCOUNT_FACTORY_CONTRACT",
      msg: {
        user: {
          name: "alice"
        }
      }
    }
  })
}
```

**Response:**

```json
{
  "index": 42,
  "name": "alice",
  "accounts": {
    "100": "0xabcd...1234"
  },
  "keys": {
    "A1B2C3...": {
      "ethereum": "0x1234...abcd"
    }
  }
}
```

You can also look up by index: `{ "user": { "index": 42 } }`.

### 3.8 Query users by key

Search for users by public key or key hash. Useful when you know a user's key but not their index or username.

```graphql
query {
  users(publicKeyHash: "A1B2C3D4...64HEX", first: 10) {
    nodes {
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
        createdTxHash
        createdAt
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

Filter by `publicKeyHash` or by `publicKey` (the raw key value). The key hash is computed differently depending on key type:

| Key type    | Input to SHA-256                                            |
| ----------- | ----------------------------------------------------------- |
| `ETHEREUM`  | UTF-8 bytes of the lowercase hex address (with `0x` prefix) |
| `SECP256K1` | Compressed public key bytes (33 bytes)                      |
| `SECP256R1` | WebAuthn credential ID bytes                                |

The resulting hash is hex-encoded in uppercase.

### 3.9 Query accounts

```graphql
query {
  accounts(userIndex: 0, first: 10) {
    nodes {
      accountIndex
      address
      createdBlockHeight
      createdTxHash
      createdAt
      users {
        userIndex
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

Filter by `userIndex` to get all accounts for a specific user, or by `address` for a specific account.

### 3.10 Query bridge deposit delivery

Deposits bridged in from another chain (see [§3.1](#31-register-user) Step 2) arrive as Hyperlane **warp route** transfers: the warp contract on the origin chain dispatches a Hyperlane message, an off-chain relayer submits it to the Dango **mailbox** contract, and the mailbox verifies the message, records it as delivered, and credits the bridged tokens to the recipient account — all within a single transaction.

To monitor a pending deposit, query the mailbox's `delivered` method with the Hyperlane message ID:

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "MAILBOX_CONTRACT",
      msg: {
        delivered: {
          message_id: "A1B2C3D4...64HEX"
        }
      }
    }
  })
}
```

| Parameter    | Type      | Description                                           |
| ------------ | --------- | ----------------------------------------------------- |
| `message_id` | `Hash256` | Hyperlane message ID of the warp transfer (see below) |

**Response:** `true` if the message has been delivered, `false` otherwise. Delivery and crediting are atomic, so once the query returns `true` the tokens are spendable in the recipient account — and if this was the activating deposit of a new master account, the account is active. Deliveries are recorded permanently, so the result never reverts to `false`.

**Obtaining the message ID.** The message ID is the keccak256 hash of the encoded Hyperlane message. The mailbox on the origin chain returns it from the `dispatch()` call and emits it in the `DispatchId` event, so it can be read from the origin-chain transaction receipt.

**Encoding.** `Hash256` is parsed strictly as 64 hex characters, **uppercase, without the `0x` prefix** (see [§10.2](#102-identifiers)). EVM tooling typically displays message IDs as `0x`-prefixed lowercase — strip the prefix and uppercase the rest, otherwise the query fails with a deserialization error.

For continuous monitoring, poll this query at a block interval using the `queryApp` subscription ([§8.4](#84-contract-query-polling)). Alternatively, subscribe to the event stream ([§8.6](#86-event-stream)) filtered to the `mailbox_process_id` event, which the mailbox emits upon delivery; the filter value must use the same uppercase, unprefixed format:

```graphql
subscription {
  events(
    filter: [
      {
        type: "mailbox_process_id",
        data: [
          {
            path: ["message_id"],
            checkMode: EQUAL,
            value: ["A1B2C3D4...64HEX"]
          }
        ]
      }
    ]
  ) {
    type
    data
    blockHeight
    createdAt
  }
}
```

## 4. Market data

### 4.1 Global parameters

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        param: {}
      }
    }
  })
}
```

**Response:**

```json
{
  "max_unlocks": 5,
  "max_open_orders": 50,
  "max_action_batch_size": 5,
  "maker_fee_rates": {
    "base": "0.000000",
    "tiers": {}
  },
  "taker_fee_rates": {
    "base": "0.001000",
    "tiers": {}
  },
  "protocol_fee_rate": "0.100000",
  "liquidation_fee_rate": "0.010000",
  "liquidation_buffer_ratio": "0.000000",
  "funding_period": "3600",
  "vault_total_weight": "10.000000",
  "vault_cooldown_period": "604800",
  "referral_active": true,
  "min_referrer_volume": "0.000000",
  "referrer_commission_rates": {
    "base": "0.000000",
    "tiers": {}
  }
}
```

| Field                       | Type            | Description                                                                      |
| --------------------------- | --------------- | -------------------------------------------------------------------------------- |
| `max_unlocks`               | `usize`         | Max concurrent vault unlock requests per user                                    |
| `max_open_orders`           | `usize`         | Max resting limit orders per user (all pairs)                                    |
| `max_action_batch_size`     | `usize`         | Max actions in a single [`batch_update_orders`](#66-batch-update-orders) message |
| `maker_fee_rates`           | `RateSchedule`  | Volume-tiered maker fee rates                                                    |
| `taker_fee_rates`           | `RateSchedule`  | Volume-tiered taker fee rates                                                    |
| `protocol_fee_rate`         | `Dimensionless` | Fraction of trading fees routed to treasury                                      |
| `liquidation_fee_rate`      | `Dimensionless` | Insurance fund fee on liquidations                                               |
| `liquidation_buffer_ratio`  | `Dimensionless` | Post-liquidation equity buffer above maintenance margin                          |
| `funding_period`            | `Duration`      | Interval between funding collections                                             |
| `vault_total_weight`        | `Dimensionless` | Sum of all pairs' vault liquidity weights                                        |
| `vault_cooldown_period`     | `Duration`      | Waiting time before vault withdrawal release                                     |
| `referral_active`           | `bool`          | Whether the referral commission system is active                                 |
| `min_referrer_volume`       | `UsdValue`      | Minimum lifetime volume to become a referrer                                     |
| `referrer_commission_rates` | `RateSchedule`  | Volume-tiered referrer commission rates                                          |

A `RateSchedule` has two fields: `base` (the default rate) and `tiers` (a map of volume threshold to rate; highest qualifying tier wins).

For fee mechanics, see [Order matching §8](2-order-matching.md#8-trading-fees).

### 4.2 Global state

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        state: {}
      }
    }
  })
}
```

**Response:**

```json
{
  "last_funding_time": "1700000000.123456789",
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

### 4.3 Pair parameters

**All pairs:**

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        pair_params: {
          start_after: null,
          limit: 30
        }
      }
    }
  })
}
```

**Single pair:**

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        pair_param: {
          pair_id: "perp/btcusd"
        }
      }
    }
  })
}
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
  "max_limit_price_deviation": "0.100000",
  "max_market_slippage": "0.100000",
  "bucket_sizes": ["1.000000", "5.000000", "10.000000"]
}
```

| Field                       | Type            | Description                                                        |
| --------------------------- | --------------- | ------------------------------------------------------------------ |
| `tick_size`                 | `UsdPrice`      | Minimum price increment for limit orders                           |
| `min_order_size`            | `UsdValue`      | Minimum notional value (reduce-only exempt)                        |
| `max_abs_oi`                | `Quantity`      | Maximum open interest per side                                     |
| `max_abs_funding_rate`      | `FundingRate`   | Daily funding rate cap                                             |
| `initial_margin_ratio`      | `Dimensionless` | Margin to open (e.g. 0.05 = 20x max leverage)                      |
| `maintenance_margin_ratio`  | `Dimensionless` | Margin to stay open (liquidation threshold)                        |
| `impact_size`               | `UsdValue`      | Notional for impact price calculation                              |
| `vault_liquidity_weight`    | `Dimensionless` | Vault allocation weight for this pair                              |
| `vault_half_spread`         | `Dimensionless` | Half the vault's bid-ask spread                                    |
| `vault_max_quote_size`      | `Quantity`      | Maximum vault resting size per side                                |
| `max_limit_price_deviation` | `Dimensionless` | Max symmetric deviation of a limit price from oracle at submission |
| `max_market_slippage`       | `Dimensionless` | Max `max_slippage` a user may set on a market or TP/SL child order |
| `bucket_sizes`              | `[UsdPrice]`    | Price bucket granularities for depth queries                       |

For the relationship between margin ratios and leverage, see [Risk §2](6-risk.md).

### 4.4 Pair state

**All pairs:**

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        pair_states: {
          start_after: null,
          limit: 30
        }
      }
    }
  })
}
```

**Single pair:**

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        pair_state: {
          pair_id: "perp/btcusd"
        }
      }
    }
  })
}
```

**Response:**

```json
{
  "long_oi": "12500.000000",
  "short_oi": "10300.000000",
  "funding_per_unit": "0.000123",
  "funding_rate": "0.000050"
}
```

| Field              | Type             | Description                                         |
| ------------------ | ---------------- | --------------------------------------------------- |
| `long_oi`          | `Quantity`       | Total long open interest                            |
| `short_oi`         | `Quantity`       | Total short open interest                           |
| `funding_per_unit` | `FundingPerUnit` | Cumulative funding accumulator                      |
| `funding_rate`     | `FundingRate`    | Current per-day funding rate (positive = longs pay) |

For funding mechanics, see [Funding](3-funding.md).

### 4.5 Order book depth

Query aggregated order book depth at a given price bucket granularity:

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
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
    "64990.000000": {
      "size": "12.500000",
      "notional": "812375.000000"
    },
    "64980.000000": {
      "size": "8.200000",
      "notional": "532836.000000"
    }
  },
  "asks": {
    "65010.000000": {
      "size": "10.000000",
      "notional": "650100.000000"
    },
    "65020.000000": {
      "size": "5.500000",
      "notional": "357610.000000"
    }
  }
}
```

Each level contains:

| Field      | Type       | Description                       |
| ---------- | ---------- | --------------------------------- |
| `size`     | `Quantity` | Absolute order size in the bucket |
| `notional` | `UsdValue` | USD notional (size × price)       |

### 4.6 Pair statistics

**All pairs:**

```graphql
query {
  allPerpsPairStats {
    pairId
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
  perpsPairStats(pairId: "perp/btcusd") {
    pairId
    currentPrice
    price24HAgo
    volume24H
    priceChange24H
  }
}
```

| Field            | Type          | Description                                        |
| ---------------- | ------------- | -------------------------------------------------- |
| `pairId`         | `String!`     | Pair identifier                                    |
| `currentPrice`   | `BigDecimal`  | Current price (nullable)                           |
| `price24HAgo`    | `BigDecimal`  | Price 24 hours ago (nullable)                      |
| `volume24H`      | `BigDecimal!` | 24h trading volume in USD                          |
| `priceChange24H` | `BigDecimal`  | 24h price change percentage (e.g. `5.25` = +5.25%) |

### 4.7 Historical candles

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
    pageInfo {
      hasNextPage
      endCursor
    }
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

### 4.8 Fees and revenue

```graphql
query {
  perpsFeesAndRevenue(
    from: "2026-01-01T00:00:00Z",
    to: "2026-02-01T00:00:00Z"
  ) {
    from
    to
    feeEventsCount
    protocolFee
    vaultFee
    refereeRebate
    referrerPayout
    volumeUsd
  }
}
```

| Parameter | Type        | Description                    |
| --------- | ----------- | ------------------------------ |
| `from`    | `DateTime!` | Window lower bound (inclusive) |
| `to`      | `DateTime!` | Window upper bound (inclusive) |

`from` must be less than or equal to `to`.

**Resolution.** Windows shorter than 3 days are served from per-block rows with microsecond-precise bounds. Windows of 3 days or more are served from the `perps_fees_hourly` materialized view; bounds are snapped to the enclosing hours, so a request that overlaps partial hours at either end includes those hours' full aggregates.

**PerpsFeesAndRevenue fields:**

| Field            | Type          | Description                                                     |
| ---------------- | ------------- | --------------------------------------------------------------- |
| `from`           | `String!`     | Lower bound echoed back as ISO 8601                             |
| `to`             | `String!`     | Upper bound echoed back as ISO 8601                             |
| `feeEventsCount` | `Int!`        | Number of `FeeDistributed` events aggregated in the window      |
| `protocolFee`    | `BigDecimal!` | Protocol fee accrued (USD)                                      |
| `vaultFee`       | `BigDecimal!` | Vault fee accrued (USD), already net of referral commissions    |
| `refereeRebate`  | `BigDecimal!` | Referral commissions paid back to referees (USD)                |
| `referrerPayout` | `BigDecimal!` | Referral commissions paid out to referrers (USD)                |
| `volumeUsd`      | `BigDecimal!` | USD notional volume from `OrderFilled` and `Deleveraged` events |

Total protocol revenue over the window is `protocolFee + vaultFee`. The total fee paid by users is `protocolFee + vaultFee + refereeRebate + referrerPayout`; `refereeRebate` and `referrerPayout` are informational totals of referral commissions distributed.

This query backs the Dango entry on DefiLlama: <https://defillama.com/protocol/dango>.

## 5. User state and orders

### 5.1 User state

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        user_state: {
          user: "0x1234...abcd"
        }
      }
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
      "entry_funding_per_unit": "0.000100",
      "conditional_order_above": {
        "order_id": "55",
        "size": "-0.500000",
        "trigger_price": "70000.000000",
        "max_slippage": "0.020000"
      },
      "conditional_order_below": null
    }
  },
  "unlocks": [],
  "reserved_margin": "500.000000",
  "open_order_count": 2
}
```

| Field              | Type                    | Description                              |
| ------------------ | ----------------------- | ---------------------------------------- |
| `margin`           | `UsdValue`              | Deposited margin (USD)                   |
| `vault_shares`     | `Uint128`               | Vault liquidity shares owned             |
| `positions`        | `Map<PairId, Position>` | Open positions by pair                   |
| `unlocks`          | `[Unlock]`              | Pending vault withdrawals                |
| `reserved_margin`  | `UsdValue`              | Margin reserved for resting limit orders |
| `open_order_count` | `usize`                 | Number of resting limit orders           |

**Position:**

| Field                     | Type                     | Description                                       |
| ------------------------- | ------------------------ | ------------------------------------------------- |
| `size`                    | `Quantity`               | Position size (positive = long, negative = short) |
| `entry_price`             | `UsdPrice`               | Average entry price                               |
| `entry_funding_per_unit`  | `FundingPerUnit`         | Funding accumulator at last modification          |
| `conditional_order_above` | `ConditionalOrder\|null` | TP/SL that triggers when oracle >= trigger_price  |
| `conditional_order_below` | `ConditionalOrder\|null` | TP/SL that triggers when oracle <= trigger_price  |

**ConditionalOrder** (embedded in Position):

| Field           | Type             | Description                                                          |
| --------------- | ---------------- | -------------------------------------------------------------------- |
| `order_id`      | `OrderId`        | Internal ID for price-time priority                                  |
| `size`          | `Quantity\|null` | Size to close (sign opposes position); `null` closes entire position |
| `trigger_price` | `UsdPrice`       | Oracle price that activates this order                               |
| `max_slippage`  | `Dimensionless`  | Slippage tolerance for the market order at trigger                   |

**Unlock:**

| Field               | Type        | Description             |
| ------------------- | ----------- | ----------------------- |
| `end_time`          | `Timestamp` | When cooldown completes |
| `amount_to_release` | `UsdValue`  | USD value to release    |

Returns `null` if the user has no state.

**Enumerate all user states (paginated):**

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        user_states: {
          start_after: null,
          limit: 10
        }
      }
    }
  })
}
```

Returns: `{ "<address>": <UserState>, ... }`

### 5.2 Open orders

Query all resting limit orders for a user:

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        orders_by_user: {
          user: "0x1234...abcd"
        }
      }
    }
  })
}
```

**Response:**

```json
{
  "42": {
    "pair_id": "perp/btcusd",
    "size": "0.500000",
    "limit_price": "63000.000000",
    "reduce_only": false,
    "reserved_margin": "1575.000000",
    "created_at": "1700000000"
  }
}
```

The response is a map of `OrderId` → order details. This query returns only resting **limit orders**. Conditional (TP/SL) orders are stored on the position itself and can be queried via `user_state` (see [§5.1](#51-user-state), `conditional_order_above` / `conditional_order_below` fields).

| Field             | Type        | Description                                         |
| ----------------- | ----------- | --------------------------------------------------- |
| `pair_id`         | `PairId`    | Trading pair                                        |
| `size`            | `Quantity`  | Order size (positive = buy, negative = sell)        |
| `limit_price`     | `UsdPrice`  | Limit price                                         |
| `reduce_only`     | `bool`      | Whether the order only reduces an existing position |
| `reserved_margin` | `UsdValue`  | Margin reserved for this order                      |
| `created_at`      | `Timestamp` | Creation time                                       |

### 5.3 Single order

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        order: {
          order_id: "42"
        }
      }
    }
  })
}
```

**Response:**

```json
{
  "user": "0x1234...abcd",
  "pair_id": "perp/btcusd",
  "size": "0.500000",
  "limit_price": "63000.000000",
  "reduce_only": false,
  "reserved_margin": "1575.000000",
  "created_at": "1700000000"
}
```

| Field             | Type        | Description                                         |
| ----------------- | ----------- | --------------------------------------------------- |
| `user`            | `Addr`      | Order owner                                         |
| `pair_id`         | `PairId`    | Trading pair                                        |
| `size`            | `Quantity`  | Order size (positive = buy, negative = sell)        |
| `limit_price`     | `UsdPrice`  | Limit price                                         |
| `reduce_only`     | `bool`      | Whether the order only reduces an existing position |
| `reserved_margin` | `UsdValue`  | Margin reserved for this order                      |
| `created_at`      | `Timestamp` | Creation time                                       |

Returns `null` if the order does not exist.

### 5.4 Trading volume

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        volume: {
          user: "0x1234...abcd",
          since: null
        }
      }
    }
  })
}
```

| Parameter | Type         | Description                            |
| --------- | ------------ | -------------------------------------- |
| `user`    | `Addr`       | Account address                        |
| `since`   | `Timestamp?` | Start time; `null` for lifetime volume |

Returns a `UsdValue` string (e.g. `"1250000.000000"`).

### 5.5 Trade history

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
    pageInfo {
      hasNextPage
      endCursor
    }
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
  "realized_funding": "0.000000",
  "fee": "6.500000",
  "client_order_id": "42",
  "fill_id": "17",
  "is_maker": false
}
```

### 5.6 Extended user state

Query user state with additional computed fields (equity, available margin, maintenance margin, and per-position unrealized PnL/funding):

```graphql
query {
  queryApp(request: {
    wasm_smart: {
      contract: "PERPS_CONTRACT",
      msg: {
        user_state_extended: {
          user: "0x1234...abcd",
          include_equity: true,
          include_available_margin: true,
          include_maintenance_margin: true,
          include_unrealized_pnl: true,
          include_unrealized_funding: true,
          include_liquidation_price: true
        }
      }
    }
  })
}
```

| Parameter                    | Type   | Description                                              |
| ---------------------------- | ------ | -------------------------------------------------------- |
| `user`                       | `Addr` | Account address                                          |
| `include_equity`             | `bool` | Compute and return the user's equity                     |
| `include_available_margin`   | `bool` | Compute and return the user's free margin                |
| `include_maintenance_margin` | `bool` | Compute and return the user's maintenance margin         |
| `include_unrealized_pnl`     | `bool` | Compute and return per-position unrealized PnL           |
| `include_unrealized_funding` | `bool` | Compute and return per-position unrealized funding costs |
| `include_liquidation_price`  | `bool` | Compute and return per-position liquidation price        |

**Response:**

```json
{
  "margin": "10000.000000",
  "vault_shares": "0",
  "unlocks": [],
  "reserved_margin": "500.000000",
  "open_order_count": 2,
  "equity": "10250.000000",
  "available_margin": "8625.000000",
  "maintenance_margin": "1875.000000",
  "positions": {
    "perp/ethusd": {
      "size": "5.000000",
      "entry_price": "2000.000000",
      "entry_funding_per_unit": "0.000000",
      "conditional_order_above": null,
      "conditional_order_below": null,
      "unrealized_pnl": "250.000000",
      "unrealized_funding": "0.000000",
      "liquidation_price": "1052.631578"
    }
  }
}
```

| Field                | Type                            | Description                                                                                               |
| -------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `margin`             | `UsdValue`                      | The user's deposited margin                                                                               |
| `vault_shares`       | `Uint128`                       | Vault shares owned by this user                                                                           |
| `unlocks`            | `[Unlock]`                      | Pending vault withdrawal cooldowns                                                                        |
| `reserved_margin`    | `UsdValue`                      | Margin reserved for resting limit orders                                                                  |
| `open_order_count`   | `usize`                         | Number of resting limit orders                                                                            |
| `equity`             | `UsdValue\|null`                | margin + unrealized PnL − unrealized funding; `null` if not requested                                     |
| `available_margin`   | `UsdValue\|null`                | margin − initial margin requirements − reserved margin; `null` if not requested                           |
| `maintenance_margin` | `UsdValue\|null`                | sum of `\|size\| * oracle_price * maintenance_margin_ratio` across all positions; `null` if not requested |
| `positions`          | `Map<PairId, PositionExtended>` | Open positions with optional computed data (see below)                                                    |

**PositionExtended:**

| Field                     | Type                     | Description                                                                                                                   |
| ------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `size`                    | `Quantity`               | Positive = long, negative = short                                                                                             |
| `entry_price`             | `UsdPrice`               | Average entry price                                                                                                           |
| `entry_funding_per_unit`  | `FundingPerUnit`         | Funding accumulator at last update                                                                                            |
| `conditional_order_above` | `ConditionalOrder\|null` | TP/SL that triggers when oracle >= trigger_price                                                                              |
| `conditional_order_below` | `ConditionalOrder\|null` | TP/SL that triggers when oracle <= trigger_price                                                                              |
| `unrealized_pnl`          | `UsdValue\|null`         | `size * (oracle_price - entry_price)`; positive = profit; `null` if not requested                                             |
| `unrealized_funding`      | `UsdValue\|null`         | `size * (current_funding_per_unit - entry_funding_per_unit)`; positive = cost; `null` if not requested                        |
| `liquidation_price`       | `UsdPrice\|null`         | Oracle price that triggers account liquidation (other prices held constant); `null` if not requested or no valid price exists |

`equity` reflects the total account value including unrealized positions. `available_margin` is the amount the user can withdraw or use for new orders. `maintenance_margin` is the minimum equity required to keep positions open — if equity falls below this threshold the account becomes liquidatable.

## 6. Trading operations

Each message is wrapped in a `Tx` as described in [§2](#2-authentication-and-transactions) and broadcast via `broadcastTxSync`.

### 6.1 Deposit margin

Deposit USDC into the trading margin account:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "deposit": {}
      }
    },
    "funds": {
      "bridge/usdc": "1000000000"
    }
  }
}
```

The deposited USDC is converted to USD at a fixed rate of \$1 per USDC and credited to `user_state.margin`. In this example, `1000000000` base units = 1,000 USDC = \$1,000.

USDC bridged in from another chain must arrive in the account before it can be deposited; to confirm delivery of a bridge transfer, see [§3.10](#310-query-bridge-deposit-delivery).

### 6.2 Withdraw margin

Withdraw USD from the trading margin account:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "withdraw": {
          "amount": "500.000000"
        }
      }
    },
    "funds": {}
  }
}
```

| Field    | Type       | Description            |
| -------- | ---------- | ---------------------- |
| `amount` | `UsdValue` | USD amount to withdraw |

The USD amount is converted to USDC at the fixed rate of \$1 per USDC (floor-rounded) and transferred to the sender.

### 6.3 Submit market order

Buy or sell at the best available prices with a slippage tolerance:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "0.100000",
          "kind": {
            "market": {
              "max_slippage": "0.010000"
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

| Field          | Type            | Description                                                |
| -------------- | --------------- | ---------------------------------------------------------- |
| `pair_id`      | `PairId`        | Trading pair (e.g. `"perp/btcusd"`)                        |
| `size`         | `Quantity`      | Contract size — **positive = buy, negative = sell**        |
| `max_slippage` | `Dimensionless` | Maximum slippage as a fraction of oracle price (0.01 = 1%) |
| `reduce_only`  | `bool`          | If `true`, only the position-closing portion executes      |
| `tp`           | `ChildOrder?`   | Optional take-profit child order (see below)               |
| `sl`           | `ChildOrder?`   | Optional stop-loss child order (see below)                 |

Market orders execute immediately (IOC behavior). Any unfilled remainder is discarded. If nothing fills, the transaction reverts.

**Child orders (TP/SL):** When `tp` or `sl` is provided, a conditional order is automatically attached to the resulting position after fill. See [ChildOrder](#childorder) in the types reference.

```json
{
  "tp": {
    "trigger_price": "70000.000000",
    "max_slippage": "0.020000",
    "size": null
  },
  "sl": {
    "trigger_price": "60000.000000",
    "max_slippage": "0.020000",
    "size": null
  }
}
```

| Field           | Type             | Description                                                          |
| --------------- | ---------------- | -------------------------------------------------------------------- |
| `trigger_price` | `UsdPrice`       | Oracle price that activates this order                               |
| `max_slippage`  | `Dimensionless`  | Slippage tolerance for the market order at trigger                   |
| `size`          | `Quantity\|null` | Size to close (sign opposes position); `null` closes entire position |

For order matching mechanics, see [Order matching](2-order-matching.md).

### 6.4 Submit limit order

Place a resting order on the book:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "submit_order": {
          "pair_id": "perp/btcusd",
          "size": "-0.500000",
          "kind": {
            "limit": {
              "limit_price": "65000.000000",
              "time_in_force": "GTC",
              "client_order_id": "42"
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

| Field             | Type             | Description                                                            |
| ----------------- | ---------------- | ---------------------------------------------------------------------- |
| `limit_price`     | `UsdPrice`       | Limit price — must be aligned to `tick_size`                           |
| `time_in_force`   | `TimeInForce`    | `"GTC"` (default), `"IOC"`, or `"POST"` — see below                    |
| `client_order_id` | `ClientOrderId?` | Optional caller-assigned id for in-flight cancel — see below           |
| `reduce_only`     | `bool`           | If `true`, only position-closing portion is kept                       |
| `tp`              | `ChildOrder?`    | Optional take-profit child order (see [§6.3](#63-submit-market-order)) |
| `sl`              | `ChildOrder?`    | Optional stop-loss child order (see [§6.3](#63-submit-market-order))   |

**Time-in-force options:**

- **GTC** (Good-Til-Canceled, default): the matching portion fills immediately; any unfilled remainder is stored on the book. Margin is reserved for the unfilled portion.
- **IOC** (Immediate-Or-Cancel): fills as much as possible against the book, then discards any unfilled remainder. Errors if nothing fills at all.
- **POST** (Post-Only): the entire order is placed on the book without matching. Rejected if the limit price would cross the best offer on the opposite side.

**Client order id:**

`client_order_id` is a caller-assigned `Uint64` that lets an algo trader cancel an order in the same block it was submitted, without round-tripping through the server response to learn the system-assigned `OrderId`. Cancel via [`CancelOrderRequest::OneByClientOrderId`](#65-cancel-order).

- Uniqueness scope: per-sender, across the sender's _active_ (resting) limit orders only. The contract does not remember client order ids of orders that have been canceled or filled, so they can be reused freely.
- Submitting a second order with a `client_order_id` that the sender already has on the book fails with `duplicate_data`.
- Not allowed with `time_in_force: "IOC"` — IOC never enters the book, so the alias would be unreachable. Submission is rejected with a clear error.

### 6.5 Cancel order

**Cancel a single order by system-assigned `OrderId`:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_order": {
          "one": "42"
        }
      }
    },
    "funds": {}
  }
}
```

**Cancel a single order by caller-assigned `ClientOrderId`:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_order": {
          "one_by_client_order_id": "42"
        }
      }
    },
    "funds": {}
  }
}
```

This resolves to the active order owned by the sender that carries the given `client_order_id` (see [§6.4](#64-submit-limit-order)). It bails if no such order exists. The lookup is per-sender, so two traders can independently use the same `client_order_id` value without colliding.

Pattern: an algo trader can submit and cancel in the same block by reusing the `client_order_id` they assigned at submission, without waiting for the server response.

**Cancel all orders:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_order": "all"
      }
    },
    "funds": {}
  }
}
```

Cancellation releases reserved margin and decrements `open_order_count`.

### 6.6 Batch update orders

Apply a sequence of submit and cancel actions atomically. Actions execute in order; later actions observe the state written by earlier ones. If any action fails, the whole message reverts and no partial state is persisted.

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "batch_update_orders": [
          { "cancel": "all" },
          {
            "submit": {
              "pair_id": "perp/btcusd",
              "size": "0.100000",
              "kind": {
                "limit": {
                  "limit_price": "64000.000000",
                  "time_in_force": "POST",
                  "client_order_id": "1"
                }
              },
              "reduce_only": false
            }
          },
          {
            "submit": {
              "pair_id": "perp/btcusd",
              "size": "-0.100000",
              "kind": {
                "limit": {
                  "limit_price": "66000.000000",
                  "time_in_force": "POST",
                  "client_order_id": "2"
                }
              },
              "reduce_only": false
            }
          }
        ]
      }
    },
    "funds": {}
  }
}
```

The payload is a JSON array of `SubmitOrCancelOrderRequest` values. Each entry is one of:

- `{ "submit": { … } }` — same shape as [`submit_order`](#64-submit-limit-order) (every field of `SubmitOrderRequest`).
- `{ "cancel": <CancelOrderRequest> }` — any variant of [`CancelOrderRequest`](#103-enums): `{ "one": "..." }`, `{ "one_by_client_order_id": "..." }`, or the string `"all"`.

**Constraints:**

- The list must be non-empty.
- The list length must not exceed [`Param.max_action_batch_size`](#41-global-parameters); the chain rejects oversize batches before any action runs.
- Conditional (TP/SL) orders are not supported in batches — use [`submit_conditional_order`](#67-submit-conditional-order-tpsl) / [`cancel_conditional_order`](#68-cancel-conditional-order) for those.

**Atomicity:** every storage write the earlier actions made — including realized fills that mutated counterparties' state — is discarded if a later action fails. Events are emitted only for the successful-batch case; a reverting batch surfaces just the top-level transaction failure.

**Example use case — atomic book replacement:**

An algo trader refreshing quotes at a new reference price can send a single `batch_update_orders` carrying `{ "cancel": "all" }` followed by the new `submit` entries. The old orders are canceled (freeing reserved margin) before the new submits run their margin checks, and if any new submit would fail (margin check, price band, OI cap, …) the old orders are restored along with the rest of the batch.

**Reusing a `client_order_id` within one batch:** a `{ "cancel": { "one_by_client_order_id": "X" } }` entry releases the id before a later `{ "submit": { … "client_order_id": "X" } }` runs, so the same id can be rebound to a new order within a single message.

### 6.7 Submit conditional order (TP/SL)

Place a take-profit or stop-loss order that triggers when the oracle price crosses a threshold:

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
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

### 6.8 Cancel conditional order

Conditional orders are identified by `(pair_id, trigger_direction)`, not by order ID.

**Cancel a single conditional order:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_conditional_order": {
          "one": {
            "pair_id": "perp/btcusd",
            "trigger_direction": "above"
          }
        }
      }
    },
    "funds": {}
  }
}
```

**Cancel all conditional orders for a specific pair:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_conditional_order": {
          "all_for_pair": {
            "pair_id": "perp/btcusd"
          }
        }
      }
    },
    "funds": {}
  }
}
```

**Cancel all conditional orders:**

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "trade": {
        "cancel_conditional_order": "all"
      }
    },
    "funds": {}
  }
}
```

### 6.9 Liquidate (permissionless)

Force-close all positions of an undercollateralized user. This message can be sent by anyone (liquidation bots):

```json
{
  "execute": {
    "contract": "PERPS_CONTRACT",
    "msg": {
      "maintain": {
        "liquidate": {
          "user": "0x5678...ef01"
        }
      }
    },
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
    "contract": "PERPS_CONTRACT",
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
    "contract": "PERPS_CONTRACT",
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

## 8. Real-time subscriptions (graphql-ws)

All subscriptions in this section use the `graphql-ws` WebSocket transport described in [§1.2](#12-websocket). The `fullBlock` and `perpsEvents` feeds use the native WebSocket API instead — see [§12](#12-new-in-v0260-websocket-api).

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

Pushes updated candle data as new trades occur. Fields match the `PerpsCandle` type in [§4.7](#47-historical-candles).

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
    isMaker
  }
}
```

**Behavior:** On connection, cached recent trades are replayed first, then new trades stream in real-time.

| Field         | Type       | Description                                                                                              |
| ------------- | ---------- | -------------------------------------------------------------------------------------------------------- |
| `orderId`     | `String`   | Order ID that produced this fill                                                                         |
| `pairId`      | `String`   | Trading pair                                                                                             |
| `user`        | `String`   | Account address                                                                                          |
| `fillPrice`   | `String`   | Execution price                                                                                          |
| `fillSize`    | `String`   | Filled size (positive = buy, negative = sell)                                                            |
| `closingSize` | `String`   | Portion that closed existing position                                                                    |
| `openingSize` | `String`   | Portion that opened new position                                                                         |
| `realizedPnl` | `String`   | PnL realized on this fill, including funding settled on the pre-existing position; excludes trading fees |
| `fee`         | `String`   | Trading fee charged                                                                                      |
| `createdAt`   | `String`   | Timestamp (ISO 8601)                                                                                     |
| `blockHeight` | `Int`      | Block in which the trade occurred                                                                        |
| `tradeIdx`    | `Int`      | Index within the block                                                                                   |
| `isMaker`     | `Boolean?` | True for the maker side of a match, false for the taker side; null for trades executed before v0.16.0    |

### 8.3 Perps events

Perps-contract events are streamed over the native WebSocket API, on the `perpsEvents` channel — see [§12.2](#122-channels). For deep history, use the `perpsEvents` query ([§5.5](#55-trade-history)).

### 8.4 Contract query polling

Poll any contract query at a regular block interval:

```graphql
subscription {
  queryApp(
    request: {
      wasm_smart: {
        contract: "PERPS_CONTRACT",
        msg: {
          user_state: {
            user: "0x1234...abcd"
          }
        }
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

### 8.5 Block stream

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

### 8.6 Event stream

Subscribe to events with optional filtering:

```graphql
subscription {
  events(
    sinceBlockHeight: 100000,
    filter: [
      {
        type: "order_filled",
        data: [
          {
            path: ["user"],
            checkMode: EQUAL,
            value: ["0x1234...abcd"]
          }
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

The perps contract emits the following events. These can be queried via `perpsEvents` ([§5.5](#55-trade-history)) or streamed in real time via the WebSocket `perpsEvents` channel ([§12.2](#122-channels)) or the generic `events` subscription ([§8.6](#86-event-stream)).

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

| Event             | Fields                                                                                                                                                                            | Description                     |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `order_filled`    | `order_id`, `pair_id`, `user`, `fill_price`, `fill_size`, `closing_size`, `opening_size`, `realized_pnl`, `realized_funding?`, `fee`, `client_order_id?`, `fill_id?`, `is_maker?` | Order partially or fully filled |
| `order_persisted` | `order_id`, `pair_id`, `user`, `limit_price`, `size`, `client_order_id?`                                                                                                          | Limit order placed on book      |
| `order_removed`   | `order_id`, `pair_id`, `user`, `reason`, `client_order_id?`                                                                                                                       | Order removed from book         |

`client_order_id` is `null` if the order was submitted without one. Off-chain consumers can use it to correlate fills, persistence, and removal with the originally-submitted client id.

`fill_id` groups the two sides of a single order-book match. When a taker crosses a resting maker, two `order_filled` events are emitted — one for each side — and both carry the same `fill_id`. Successive matches use consecutive ids (strictly increasing), so a taker that crosses two makers in the same transaction produces four events with two distinct `fill_id` values. `fill_id` is `null` for trades executed before v0.15.0 — fill IDs were not assigned prior to that release. Not emitted for ADL fills, which use the [`deleveraged` and `liquidated` events](#liquidation-events) instead.

`is_maker` is `true` for the maker side of a match and `false` for the taker side. Within a single match's pair of `order_filled` events (sharing one `fill_id`), exactly one carries `is_maker = true` and one carries `is_maker = false`. `is_maker` is `null` for trades executed before v0.16.0 — the maker/taker flag was not recorded prior to that release.

`realized_pnl` on `order_filled` and `deleveraged` (and `adl_realized_pnl` on `liquidated`) reports the closing PnL on the fill — price movement on the closed portion. The funding settled on the user's pre-existing position immediately before the fill is reported separately as `realized_funding` (or `adl_realized_funding` on `liquidated`).

`realized_funding` is `null` for events emitted before v0.17.0 — funding was bundled into `realized_pnl` (and `adl_realized_pnl`) prior to that release. From v0.17.0 onward the field is always present and `realized_pnl + realized_funding` equals the pre-v0.17.0 lump sum.

Trading fees are reported separately in the `fee` field on `order_filled`; ADL and deleverage fills incur no trading fees.

### Conditional order events

| Event                         | Fields                                                                          | Description                   |
| ----------------------------- | ------------------------------------------------------------------------------- | ----------------------------- |
| `conditional_order_placed`    | `pair_id`, `user`, `trigger_price`, `trigger_direction`, `size`, `max_slippage` | TP/SL order created           |
| `conditional_order_triggered` | `pair_id`, `user`, `trigger_price`, `trigger_direction`, `oracle_price`         | TP/SL triggered by price move |
| `conditional_order_removed`   | `pair_id`, `user`, `trigger_direction`, `reason`                                | TP/SL removed                 |

### Liquidation events

| Event              | Fields                                                                                  | Description                      |
| ------------------ | --------------------------------------------------------------------------------------- | -------------------------------- |
| `liquidated`       | `user`, `pair_id`, `adl_size`, `adl_price`, `adl_realized_pnl`, `adl_realized_funding?` | Position liquidated in a pair    |
| `deleveraged`      | `user`, `pair_id`, `closing_size`, `fill_price`, `realized_pnl`, `realized_funding?`    | Counter-party hit by ADL         |
| `bad_debt_covered` | `liquidated_user`, `amount`, `insurance_fund_remaining`                                 | Insurance fund absorbed bad debt |

### ReasonForOrderRemoval

| Value                    | Description                                                                  |
| ------------------------ | ---------------------------------------------------------------------------- |
| `filled`                 | Order fully filled                                                           |
| `canceled`               | User voluntarily canceled                                                    |
| `position_closed`        | Position was closed (conditional orders only)                                |
| `self_trade_prevention`  | Order crossed user's own order on the opposite side                          |
| `liquidated`             | User was liquidated                                                          |
| `deleveraged`            | User was hit by auto-deleveraging                                            |
| `slippage_exceeded`      | Conditional order triggered but could not fill within slippage               |
| `price_band_violation`   | Resting price drifted outside the per-pair band before match                 |
| `slippage_cap_tightened` | Conditional order's stored max_slippage now exceeds the pair's tightened cap |

For liquidation and ADL mechanics, see [Liquidation & ADL](4-liquidation-and-adl.md).

## 10. Types reference

### 10.1 Numeric types

All numeric types are **signed fixed-point decimals with 6 decimal places**, built on [`dango_types::Number`](https://github.com/left-curve/left-curve/blob/main/dango/exchange/types/src/typed_number.rs). They are serialized as strings:

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
| `ClientOrderId`      | `Uint64` (caller-assigned)      | `"42"`                           |
| `FillId`             | `Uint64` (per-match identifier) | `"17"`                           |
| `Addr`               | Hex address                     | `"0x1234...abcd"`                |
| `Hash256`            | 64-char uppercase hex           | `"A1B2C3D4E5F6..."`              |
| `UserIndex`          | `u32`                           | `0`                              |
| `AccountIndex`       | `u32`                           | `1`                              |
| `Username`           | 1–15 chars, `[a-z0-9_]`         | `"alice"`                        |
| `Timestamp`          | Seconds since epoch (decimal)   | `"1700000000.123456789"`         |
| `Duration`           | Seconds (decimal)               | `"3600"` (1 hour)                |

`Timestamp` and `Duration` are encoded as fixed-point decimal strings with up to 9 fractional digits (nanosecond precision); trailing zeros are elided. So `"1700000000"`, `"1700000000.5"`, and `"1700000000.123456789"` are all valid `Timestamp` values.

### 10.3 Enums

**OrderKind:**

```json
{
  "market": {
    "max_slippage": "0.010000"
  }
}
```

```json
{
  "limit": {
    "limit_price": "65000.000000",
    "time_in_force": "GTC",
    "client_order_id": "42"
  }
}
```

`client_order_id` is optional. Defaults to `null` when omitted; not allowed with `time_in_force: "IOC"`.

**TimeInForce:** `"GTC"` | `"IOC"` | `"POST"` (defaults to `"GTC"` if omitted)

**TriggerDirection:**

```json
"above"
```

```json
"below"
```

**CancelOrderRequest:**

```json
{
  "one": "42"
}
```

```json
{
  "one_by_client_order_id": "42"
}
```

```json
"all"
```

**SubmitOrderRequest:**

```json
{
  "pair_id": "perp/btcusd",
  "size": "-0.500000",
  "kind": {
    "limit": {
      "limit_price": "65000.000000",
      "time_in_force": "GTC",
      "client_order_id": "42"
    }
  },
  "reduce_only": false,
  "tp": null,
  "sl": null
}
```

Same shape used by [`submit_order`](#64-submit-limit-order) and each `submit` entry in [`batch_update_orders`](#66-batch-update-orders).

**SubmitOrCancelOrderRequest:**

```json
{ "submit": { /* SubmitOrderRequest */ } }
```

```json
{ "cancel": { "one": "42" } }
```

```json
{ "cancel": "all" }
```

One action inside a [`batch_update_orders`](#66-batch-update-orders) list. Conditional (TP/SL) orders are not supported.

**CancelConditionalOrderRequest:**

```json
{
  "one": {
    "pair_id": "perp/btcusd",
    "trigger_direction": "above"
  }
}
```

```json
{
  "all_for_pair": {
    "pair_id": "perp/btcusd"
  }
}
```

```json
"all"
```

**Key:**

```json
{
  "secp256r1": "<base64>"
}
```

```json
{
  "secp256k1": "<base64>"
}
```

```json
{
  "ethereum": "0x1234...abcd"
}
```

**Credential:**

```json
{
  "standard": {
    "key_hash": "...",
    "signature": { ... }
  }
}
```

```json
{
  "session": {
    "session_info": { ... },
    "session_signature": "...",
    "authorization": { ... }
  }
}
```

**CandleInterval** (GraphQL enum):

`ONE_SECOND` | `ONE_MINUTE` | `FIVE_MINUTES` | `FIFTEEN_MINUTES` | `ONE_HOUR` | `FOUR_HOURS` | `ONE_DAY` | `ONE_WEEK`

### 10.4 Response types

**Param** (global parameters) — see [§4.1](#41-global-parameters) for all fields.

**PairParam** (per-pair parameters) — see [§4.3](#43-pair-parameters) for all fields.

**PairState:**

| Field              | Type             | Description                                         |
| ------------------ | ---------------- | --------------------------------------------------- |
| `long_oi`          | `Quantity`       | Total long open interest                            |
| `short_oi`         | `Quantity`       | Total short open interest                           |
| `funding_per_unit` | `FundingPerUnit` | Cumulative funding accumulator                      |
| `funding_rate`     | `FundingRate`    | Current per-day funding rate (positive = longs pay) |

**State** (global state) — see [§4.2](#42-global-state) for all fields.

**UserState** — see [§5.1](#51-user-state) for all fields.

**UserStateExtended** — see [§5.6](#56-extended-user-state) for all fields.

**Position:**

| Field                     | Type                     | Description                                      |
| ------------------------- | ------------------------ | ------------------------------------------------ |
| `size`                    | `Quantity`               | Positive = long, negative = short                |
| `entry_price`             | `UsdPrice`               | Average entry price                              |
| `entry_funding_per_unit`  | `FundingPerUnit`         | Funding accumulator at last update               |
| `conditional_order_above` | `ConditionalOrder\|null` | TP/SL that triggers when oracle >= trigger_price |
| `conditional_order_below` | `ConditionalOrder\|null` | TP/SL that triggers when oracle <= trigger_price |

<a id="conditionalorder"></a>**ConditionalOrder** (embedded in Position):

| Field           | Type             | Description                                                          |
| --------------- | ---------------- | -------------------------------------------------------------------- |
| `order_id`      | `OrderId`        | Internal ID for price-time priority                                  |
| `size`          | `Quantity\|null` | Size to close (sign opposes position); `null` closes entire position |
| `trigger_price` | `UsdPrice`       | Oracle price that activates this order                               |
| `max_slippage`  | `Dimensionless`  | Slippage tolerance for the market order at trigger                   |

<a id="childorder"></a>**ChildOrder** (TP/SL attached to a parent order):

| Field           | Type             | Description                                                          |
| --------------- | ---------------- | -------------------------------------------------------------------- |
| `trigger_price` | `UsdPrice`       | Oracle price that activates this order                               |
| `max_slippage`  | `Dimensionless`  | Slippage tolerance for the market order at trigger                   |
| `size`          | `Quantity\|null` | Size to close (sign opposes position); `null` closes entire position |

**Unlock:**

| Field               | Type        | Description             |
| ------------------- | ----------- | ----------------------- |
| `end_time`          | `Timestamp` | When cooldown completes |
| `amount_to_release` | `UsdValue`  | USD value to release    |

**QueryOrderResponse:**

| Field             | Type        | Description                                         |
| ----------------- | ----------- | --------------------------------------------------- |
| `user`            | `Addr`      | Order owner                                         |
| `pair_id`         | `PairId`    | Trading pair                                        |
| `size`            | `Quantity`  | Order size                                          |
| `limit_price`     | `UsdPrice`  | Limit price                                         |
| `reduce_only`     | `bool`      | Whether the order only reduces an existing position |
| `reserved_margin` | `UsdValue`  | Margin reserved for this order                      |
| `created_at`      | `Timestamp` | Creation time                                       |

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

## 11. New in v0.26.0: REST API

Plain REST endpoints for the most common operations, as an alternative to the GraphQL endpoint in [§1.1](#11-http). Each is a `POST` whose body is the raw object — no GraphQL `{query, variables}` wrapper — and whose response is the raw result JSON. They exist for bots and algo traders that expect a REST + WebSocket surface rather than GraphQL.

**Endpoint:** See [Constants](9-constants.md#endpoints).

**Headers:**

| Header         | Value              |
| -------------- | ------------------ |
| `Content-Type` | `application/json` |

### 11.1 Query

Run a read-only query against the latest finalized state. The body is a single `Query` object — the same shape the GraphQL `queryApp` takes as its `request` — and the response is the matching `QueryResponse`. Historical queries are not supported; there is no `height` parameter.

**Example — query a smart contract:**

```bash
curl -X POST https://<host>/query \
  -H 'Content-Type: application/json' \
  -d '{"wasm_smart": {"contract": "PERPS_CONTRACT", "msg": {"state": {}}}}'
```

**Response** — the `QueryResponse`, keyed by the request variant:

```json
{
  "wasm_smart": { ... }
}
```

The body accepts any `Query` variant — for example `{"app_config": {}}`, `{"balance": { ... }}`, or `{"multi": [ ... ]}` to batch several queries atomically (see [§1.4](#14-multi-query)).

### 11.2 Simulate

Dry-run an `UnsignedTx` to estimate gas. The body is the `UnsignedTx`; the response is the `TxOutcome`. Simulation skips signature verification — add **770,000 gas** (Secp256k1 verification cost) to `gas_used` when setting `gas_limit` on the final transaction (see [§2.8](#28-gas-estimation)).

**Request body** — an `UnsignedTx`:

```json
{
  "sender": "0x1234...abcd",
  "msgs": [ ... ],
  "data": {
    "user_index": 0,
    "chain_id": "dango-1",
    "nonce": 42,
    "expiry": null
  }
}
```

**Example:**

```bash
curl -X POST https://<host>/simulate \
  -H 'Content-Type: application/json' \
  -d '{"sender": "0x1234...abcd", "msgs": [ ... ], "data": { ... }}'
```

**Response** — the `TxOutcome`:

```json
{
  "gas_limit": null,
  "gas_used": 750000,
  "result": {
    "ok": [ ... ]
  }
}
```

### 11.3 Broadcast

Submit a signed `Tx` to the mempool. The body is the `Tx`; the response is the `BroadcastTxOutcome`. This is a mempool receipt, **not** block inclusion: a transaction accepted into the mempool returns `200` with `check_tx.code` equal to `0`, and a mempool-rejected transaction also returns `200` but with a non-zero `check_tx.code` (it never entered a block). Only a transport failure to the consensus node returns `500`.

**Request body** — a signed `Tx`:

```json
{
  "sender": "0x1234...abcd",
  "gas_limit": 1500000,
  "msgs": [ ... ],
  "data": {
    "user_index": 0,
    "chain_id": "dango-1",
    "nonce": 42,
    "expiry": null
  },
  "credential": {
    "standard": {
      "key_hash": "...",
      "signature": { ... }
    }
  }
}
```

**Example:**

```bash
curl -X POST https://<host>/broadcast \
  -H 'Content-Type: application/json' \
  -d '{"sender": "0x1234...abcd", "gas_limit": 1500000, "msgs": [ ... ], "data": { ... }, "credential": { ... }}'
```

**Response** — the `BroadcastTxOutcome`:

```json
{
  "tx_hash": "...",
  "check_tx": {
    "code": 0,
    "gas_used": 750000
  }
}
```

## 12. New in v0.26.0: WebSocket API

A native WebSocket endpoint for real-time feeds, distinct from the `graphql-ws` endpoint in [§1.2](#12-websocket). It follows the multiplexed, message-oriented convention used by major exchanges: one socket carries any number of subscriptions, the client opens and closes them with JSON messages, and every frame is a single JSON object.

**Endpoint:** `GET /ws` (WebSocket upgrade). See [Constants](9-constants.md#endpoints).

Two channels are served, both replacing `graphql-ws` subscriptions removed in this release:

- `fullBlock` — every finalized block (`Block` + `BlockOutcome`).
- `perpsEvents` — perps-contract events grouped per block, with filters.

### 12.1 Protocol

Client messages are tagged by `method`; server messages are tagged by `channel`. A `subscribe` carries a client-chosen integer `id` that is the subscription handle: it is echoed on the acknowledgement and on every data frame the subscription produces, and is used to `unsubscribe`. One socket can therefore carry several subscriptions — for example several `perpsEvents` feeds with different filters — that the client demultiplexes by `id`.

**Client → server:**

```json
{"method": "subscribe", "id": 1, "subscription": {"type": "perpsEvents", "pairIds": ["perp/btcusd"]}}
{"method": "subscribe", "id": 2, "subscription": {"type": "fullBlock"}}
{"method": "unsubscribe", "id": 1}
{"method": "ping", "id": 9}
```

| Field          | Type     | Description                                                |
| -------------- | -------- | ---------------------------------------------------------- |
| `method`       | `String` | `subscribe`, `unsubscribe`, or `ping`                      |
| `id`           | `Int`    | Subscription handle; required on `subscribe`/`unsubscribe` |
| `subscription` | `Object` | The feed selector; see [§12.2](#122-channels)              |

**Server → client:**

```json
{"channel": "subscriptionResponse", "id": 1, "data": {"method": "subscribe", "type": "perpsEvents"}}
{"channel": "perpsEvents", "id": 1, "data": { ... }}
{"channel": "perpsEvents", "id": 1, "error": {"code": "resync", "message": "..."}}
{"channel": "fullBlock", "id": 2, "data": { ... }}
{"channel": "pong", "id": 9}
{"channel": "error", "error": {"code": "badRequest", "message": "..."}}
```

| `channel`                 | Description                                                                                                                                             |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `subscriptionResponse`    | Acknowledges a `subscribe`/`unsubscribe`, echoing its `id`                                                                                              |
| `perpsEvents`/`fullBlock` | A frame on a subscription's channel, tagged with its `id`: a `data` payload, or an `error` that ended the feed (see [§12.3](#123-reconnect-and-errors)) |
| `pong`                    | Reply to a `ping`, echoing its `id`                                                                                                                     |
| `error`                   | A connection-level problem with no subscription to attribute it to (e.g. an unparseable frame); see [§12.3](#123-reconnect-and-errors)                  |

Each frame on a subscription's channel (`perpsEvents` / `fullBlock`) carries either a `data` payload or an `error`; a client demultiplexes by `id` and branches on which key is present. Errors are co-located this way so a feed's failure arrives on the same channel its data does — see [§12.3](#123-reconnect-and-errors).

**Heartbeat.** The server sends a WebSocket ping every 20 seconds and closes a connection it has heard nothing from for 60 seconds. A client either lets its WebSocket stack answer those pings, or sends `{"method": "ping"}` itself; either keeps an idle subscription alive.

### 12.2 Channels

#### `perpsEvents`

Stream every event emitted by the perps contract — order lifecycle, fills, liquidations, and deleveraging — grouped per block. Served from an in-memory window of recent blocks, so deep history is not available here; use the `perpsEvents` query ([§5.5](#55-trade-history)) for that.

```json
{"method": "subscribe", "id": 1, "subscription": {
  "type": "perpsEvents",
  "since": 100000,
  "eventTypes": ["order_filled", "liquidated"],
  "pairIds": ["perp/btcusd"],
  "users": ["0x1234...abcd"],
  "orderIds": ["100"],
  "clientOrderIds": ["42"]
}}
```

| Field            | Type       | Description                                                            |
| ---------------- | ---------- | ---------------------------------------------------------------------- |
| `since`          | `Int`      | Replay retained blocks from this height on connect; omit for live-only |
| `eventTypes`     | `[String]` | Keep only these event types (see [§9](#9-events-reference))            |
| `pairIds`        | `[String]` | Keep only these trading pairs                                          |
| `users`          | `[String]` | Keep only events whose `user` is one of these addresses                |
| `orderIds`       | `[String]` | Keep only events whose `order_id` is one of these                      |
| `clientOrderIds` | `[String]` | Keep only events whose `client_order_id` is one of these               |

**Filter semantics.** The five filters AND together. Omitting a filter does not filter on that field (matches everything); passing an _empty_ array matches nothing. Each value is matched verbatim against the event's canonical string form, so pass the same `0x`-prefixed address or decimal-id form the API returns elsewhere. A `client_order_id` is unique only per sender, so combine `clientOrderIds` with `users` to single out one trader's order.

Each data frame carries one block's matching events; only blocks with at least one matching event are delivered:

```json
{"channel": "perpsEvents", "id": 1, "data": {
  "blockHeight": 100001,
  "createdAt": "2026-06-18T00:00:00Z",
  "events": [
    {
      "idx": 0,
      "eventType": "order_filled",
      "user": "0x1234...abcd",
      "pairId": "perp/btcusd",
      "orderId": "100",
      "clientOrderId": "42",
      "data": { ... }
    }
  ]
}}
```

**`data` (PerpsEventsBatch) fields:**

| Field         | Type           | Description                       |
| ------------- | -------------- | --------------------------------- |
| `blockHeight` | `Int`          | Block height                      |
| `createdAt`   | `String`       | Block timestamp (ISO 8601)        |
| `events`      | `[PerpsEvent]` | The block's matching perps events |

**PerpsEvent fields:**

| Field           | Type      | Description                                                 |
| --------------- | --------- | ----------------------------------------------------------- |
| `idx`           | `Int`     | Ordinal of this event within the block                      |
| `eventType`     | `String`  | Event type name (see [§9](#9-events-reference))             |
| `user`          | `String?` | The event's subject address, if it has a `user` field       |
| `pairId`        | `String?` | The event's trading pair, if it has a `pair_id` field       |
| `orderId`       | `String?` | The event's order, if it has an `order_id` field            |
| `clientOrderId` | `String?` | The caller-assigned client order id, if it has one          |
| `data`          | `JSON`    | Raw event payload (same shape as [§9](#9-events-reference)) |

#### `fullBlock`

Stream every finalized block in full — the same `FullBlock` shape (`block` + `outcome`) the REST `/block/full/{height}` route returns, one data frame per block.

```json
{"method": "subscribe", "id": 2, "subscription": {"type": "fullBlock", "since": 100000}}
```

| Field   | Type  | Description                                                            |
| ------- | ----- | ---------------------------------------------------------------------- |
| `since` | `Int` | Replay retained blocks from this height on connect; omit for live-only |

```json
{"channel": "fullBlock", "id": 2, "data": {"block": { ... }, "outcome": { ... }}}
```

### 12.3 Reconnect and errors

Both channels are served from an in-memory window of recent blocks. Every data frame carries the block height (`blockHeight`, or `block.info.height` for `fullBlock`), so a client tracks the last height it saw and, on reconnect, resubscribes with `since` set to that height plus one. Subscriptions are not persisted across reconnects: a client that reconnects must resend its `subscribe` messages.

Problems are delivered as `error`-keyed frames. An error that concerns a specific subscription rides that subscription's own channel and `id` — an `error` sibling of the `data` frames it would otherwise send — so a client handles a feed's failure on the same channel it reads from. An error with no subscription to attribute it to (an unparseable message, or an `unsubscribe` for an unknown `id`) arrives on the dedicated `error` channel instead, carrying the offending `id` when there is one.

| `code`                | Meaning                                                                                                                                                                                                                                   |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `resync`              | `since` predates the retained window, or the feed lagged past it. The subscription ends; reconnect with a newer `since` and backfill the gap from the `perpsEvents` query ([§5.5](#55-trade-history)) or the `/block/full/*` REST routes. |
| `tooManyRequests`     | The server's subscription limit was reached.                                                                                                                                                                                              |
| `badRequest`          | The message could not be parsed, or the `id` is already in use.                                                                                                                                                                           |
| `unknownSubscription` | An `unsubscribe` referenced an `id` with no open subscription.                                                                                                                                                                            |

A subscription-scoped error (here, a terminal `resync` on the `perpsEvents` feed opened with `id: 1`):

```json
{"channel": "perpsEvents", "id": 1, "error": {"code": "resync", "message": "resync required: requested from block 100 but the oldest retained block is 900"}}
```
