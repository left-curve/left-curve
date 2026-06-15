# TypeScript SDK Inventory

## Summary
- Total Actions: 91
- Total Clients: 3
- Total Types: 178
- Total Errors: 4

## Packages

### @left-curve/sdk (sdk/typescript/dango)
Top-level SDK package: clients, transports, actions, chains, signers, and Hyperlane helpers for the Dango execution environment.

#### Clients
- `createBaseClient` — factory for the underlying client (transport + chain + signer, with `extend()`) | source: `sdk/typescript/dango/src/clients/baseClient.ts`
- `createPublicClient` — factory that extends a base client with read-only `publicActions` | source: `sdk/typescript/dango/src/clients/publicClient.ts`
- `createSignerClient` — factory that extends a base client with `publicActions + signerActions` for transaction signing | source: `sdk/typescript/dango/src/clients/signerClient.ts`

#### Transports
- `createTransport` — GraphQL HTTP/WS transport factory with batching, fetch hooks, and WS retry | source: `sdk/typescript/dango/src/transports/graphql.ts`

#### Chains
- `local` — chain config for a local devnet | source: `sdk/typescript/dango/src/chains/definitions/local.ts`
- `devnet` — chain config for the public devnet | source: `sdk/typescript/dango/src/chains/definitions/devnet.ts`
- `testnet` — chain config for the public testnet | source: `sdk/typescript/dango/src/chains/definitions/testnet.ts`
- `mainnet` — chain config for mainnet (`dango-1`) | source: `sdk/typescript/dango/src/chains/definitions/mainnet.ts`

#### Account helpers
- `computeAddress` — derive a contract address from deployer + codeHash + salt (mirrors `dango_primitives::Addr::derive`) | source: `sdk/typescript/dango/src/account/address.ts`
- `isValidAddress` — check if a string is a syntactically valid hex address | source: `sdk/typescript/dango/src/account/address.ts`
- `createAccountSalt` — build the salt used when registering a new user account | source: `sdk/typescript/dango/src/account/salt.ts`
- `createKeyHash` — sha256 a public key / credential id and uppercase-hex it | source: `sdk/typescript/dango/src/account/key.ts`
- `createSignBytes` — produce the byte payload the `grug-account` contract expects to sign | source: `sdk/typescript/dango/src/account/signature.ts`
- `toAccount` — assemble an `Account` from a `User`, account index, and address | source: `sdk/typescript/dango/src/account/accountInfo.ts`

#### Signers
- `PrivateKeySigner` — `Signer` implementation backed by a local secp256k1 key (mnemonic / private key / random) | source: `sdk/typescript/dango/src/signers/privateKey.ts`
- `createSessionSigner` — `Signer` factory that signs with a session key + stored authorization | source: `sdk/typescript/dango/src/signers/session.ts`

#### Action builders (composite, used by clients)
- `publicActions` — bundles all query action builders into a single object | source: `sdk/typescript/dango/src/actions/publicActions.ts`
- `signerActions` — bundles all mutation action builders into a single object | source: `sdk/typescript/dango/src/actions/signerActions.ts`
- `appQueryActions` / `appMutationActions` — per-domain action builders for the `app` domain | source: `sdk/typescript/dango/src/actions/app/appActions.ts`
- `accountFactoryQueryActions` / `accountFactoryMutationActions` — per-domain builders for `account-factory` | source: `sdk/typescript/dango/src/actions/account-factory/accountFactoryActions.ts`
- `dexQueryActions` / `dexMutationActions` — per-domain builders for `dex` | source: `sdk/typescript/dango/src/actions/dex/dexActions.ts`
- `gatewayQueryActions` / `gatewayMutationActions` — per-domain builders for `gateway` (namespaced under `client.gateway.*`) | source: `sdk/typescript/dango/src/actions/gateway/gatewayActions.ts`
- `indexerActions` — per-domain builders for `indexer` queries + subscriptions | source: `sdk/typescript/dango/src/actions/indexer/indexerActions.ts`
- `oracleQueryActions` — per-domain builders for `oracle` | source: `sdk/typescript/dango/src/actions/oracle/oracleActions.ts`
- `perpsQueryActions` / `perpsMutationActions` — per-domain builders for `perps` | source: `sdk/typescript/dango/src/actions/perps/perpsActions.ts`

#### Actions

##### Domain: app
Queries:
- `getAppConfig` — fetch and memoize the app-level configuration (`AppConfig`) | source: `sdk/typescript/dango/src/actions/app/queries/getAppConfig.ts`
- `getBalance` — balance of one denom for an address (returns a `number`) | source: `sdk/typescript/dango/src/actions/app/queries/getBalance.ts`
- `getBalances` — paginated `Coins` map for an address | source: `sdk/typescript/dango/src/actions/app/queries/getBalances.ts`
- `getSupply` — total supply of a single token | source: `sdk/typescript/dango/src/actions/app/queries/getSupply.ts`
- `getSupplies` — paginated map of token supplies | source: `sdk/typescript/dango/src/actions/app/queries/getSupplies.ts`
- `getCode` — fetch a stored Wasm code by hash | source: `sdk/typescript/dango/src/actions/app/queries/getCode.ts`
- `getCodes` — paginated list of stored Wasm codes | source: `sdk/typescript/dango/src/actions/app/queries/getCodes.ts`
- `getContractInfo` — fetch contract metadata by address | source: `sdk/typescript/dango/src/actions/app/queries/getContractInfo.ts`
- `getContractsInfo` — paginated map of contract metadata | source: `sdk/typescript/dango/src/actions/app/queries/getContractsInfo.ts`
- `queryWasmRaw` — raw base64 storage value at a contract key | source: `sdk/typescript/dango/src/actions/app/queries/queryWasmRaw.ts`
- `queryWasmSmart` — typed JSON query against a smart contract | source: `sdk/typescript/dango/src/actions/app/queries/queryWasmSmart.ts`
- `queryApp` — generic typed query against the app (`QueryRequest` -> `QueryResponse`) | source: `sdk/typescript/dango/src/actions/app/queries/queryApp.ts`
- `queryStatus` — chain id and latest block info | source: `sdk/typescript/dango/src/actions/app/queries/queryStatus.ts`
- `queryTx` — fetch an indexed transaction by hash | source: `sdk/typescript/dango/src/actions/app/queries/queryTx.ts`
- `simulate` — gas-simulate an unsigned tx, scaled gas back | source: `sdk/typescript/dango/src/actions/app/queries/simulate.ts`

Mutations:
- `broadcastTxSync` — broadcast a signed tx via the indexer mutation | source: `sdk/typescript/dango/src/actions/app/mutations/broadcastTxSync.ts`
- `signAndBroadcastTx` — sign messages with the client signer and broadcast | source: `sdk/typescript/dango/src/actions/app/mutations/signAndBroadcastTx.ts`
- `transfer` — send a record of `Address -> Coins` from the sender | source: `sdk/typescript/dango/src/actions/app/mutations/transfer.ts`
- `execute` — execute one or more contract messages (with funds and EIP-712 typed data) | source: `sdk/typescript/dango/src/actions/app/mutations/execute.ts`
- `instantiate` — instantiate a contract from a code hash; returns the derived address | source: `sdk/typescript/dango/src/actions/app/mutations/instantiate.ts`
- `migrate` — migrate a contract to a new code hash | source: `sdk/typescript/dango/src/actions/app/mutations/migrate.ts`
- `storeCode` — upload a Wasm code blob | source: `sdk/typescript/dango/src/actions/app/mutations/storeCode.ts`
- `storeCodeAndInstantiate` — upload code and instantiate in one tx | source: `sdk/typescript/dango/src/actions/app/mutations/storeCodeAndInstantiate.ts`
- `upgrade` — schedule a chain upgrade at a future block | source: `sdk/typescript/dango/src/actions/app/mutations/upgrade.ts`
- `configure` — update chain and app config | source: `sdk/typescript/dango/src/actions/app/mutations/configure.ts`

##### Domain: account-factory
Queries:
- `forgotUsername` — list users associated with a key hash | source: `sdk/typescript/dango/src/actions/account-factory/queries/forgotUsername.ts`
- `getAccountInfo` — account details (`AccountDetails`) for an address | source: `sdk/typescript/dango/src/actions/account-factory/queries/getAccountInfo.ts`
- `getAccountSeenNonces` — most-recent nonces consumed by an account | source: `sdk/typescript/dango/src/actions/account-factory/queries/getAccountSeenNonces.ts`
- `getAccountStatus` — current `UserStatus` for an address | source: `sdk/typescript/dango/src/actions/account-factory/queries/getAccountStatus.ts`
- `getAllAccountInfo` — paginated map of all `AccountInfo` in the factory | source: `sdk/typescript/dango/src/actions/account-factory/queries/getAllAccountInfo.ts`
- `getCodeHash` — current account contract code hash | source: `sdk/typescript/dango/src/actions/account-factory/queries/getCodeHash.ts`
- `getNextAccountIndex` — next account index for a username | source: `sdk/typescript/dango/src/actions/account-factory/queries/getNextAccountIndex.ts`
- `getUser` — `User` by index or name | source: `sdk/typescript/dango/src/actions/account-factory/queries/getUser.ts`
- `getUserKeys` — indexer-backed list of `PublicKey` for a user index | source: `sdk/typescript/dango/src/actions/account-factory/queries/getUserKeys.ts`

Mutations:
- `registerUser` — create a new user and first account in one tx | source: `sdk/typescript/dango/src/actions/account-factory/mutations/registerUser.ts`
- `registerAccount` — register an additional account for an existing user | source: `sdk/typescript/dango/src/actions/account-factory/mutations/registerAccount.ts`
- `updateKey` — insert or delete a key on the calling account | source: `sdk/typescript/dango/src/actions/account-factory/mutations/updateKey.ts`
- `updateUsername` — change the username on the calling account | source: `sdk/typescript/dango/src/actions/account-factory/mutations/updateUsername.ts`
- `createSession` — sign a `SigningSessionInfo` and return the session credential bundle | source: `sdk/typescript/dango/src/actions/account-factory/mutations/createSession.ts`

##### Domain: dex
Queries:
- `dexStatus` — whether the DEX is paused | source: `sdk/typescript/dango/src/actions/dex/queries/dexStatus.ts`
- `getOrder` — details of a single active order by id | source: `sdk/typescript/dango/src/actions/dex/queries/getOrder.ts`
- `getPair` — `PairParams` for a single base/quote pair | source: `sdk/typescript/dango/src/actions/dex/queries/getPair.ts`
- `getPairs` — paginated list of trading pairs | source: `sdk/typescript/dango/src/actions/dex/queries/getPairs.ts`
- `getPairStats` — 24h stats for one pair (from indexer) | source: `sdk/typescript/dango/src/actions/dex/queries/getPairStats.ts`
- `getAllPairStats` — 24h stats for every pair (from indexer) | source: `sdk/typescript/dango/src/actions/dex/queries/getAllPairStats.ts`
- `ordersByUser` — active orders for a user, keyed by order id | source: `sdk/typescript/dango/src/actions/dex/queries/ordersByUser.ts`
- `queryCandles` — paginated candles for a pair + interval | source: `sdk/typescript/dango/src/actions/dex/queries/candles.ts`
- `queryTrades` — paginated trades, optionally filtered by address | source: `sdk/typescript/dango/src/actions/dex/queries/trades.ts`
- `simulateSwapExactAmountIn` — quote the output of a swap with exact input | source: `sdk/typescript/dango/src/actions/dex/queries/simulateSwapExactAmountIn.ts`
- `simulateSwapExactAmountOut` — quote the input needed for a swap with exact output | source: `sdk/typescript/dango/src/actions/dex/queries/simulateSwapExactAmountOut.ts`
- `simulateWithdrawLiquidity` — coins returned for burning a given amount of LP | source: `sdk/typescript/dango/src/actions/dex/queries/simulateWithdrawLiquidity.ts`

Mutations:
- `batchUpdateOrders` — create and/or cancel multiple limit orders in one tx | source: `sdk/typescript/dango/src/actions/dex/mutations/batchUpdateOrders.ts`
- `swapExactAmountIn` — instant swap with exact input and min output | source: `sdk/typescript/dango/src/actions/dex/mutations/swapExactAmountIn.ts`
- `swapExactAmountOut` — instant swap with exact output, refunding excess input | source: `sdk/typescript/dango/src/actions/dex/mutations/swapExactAmountOut.ts`
- `provideLiquidity` — add liquidity to a pair | source: `sdk/typescript/dango/src/actions/dex/mutations/provideLiquidity.ts`
- `withdrawLiquidity` — withdraw liquidity from a pair | source: `sdk/typescript/dango/src/actions/dex/mutations/withdrawLiquidity.ts`

##### Domain: perps
Queries:
- `getPerpsUserState` — `PerpsUserState` for a user | source: `sdk/typescript/dango/src/actions/perps/queries/getUserState.ts`
- `getPerpsUserStateExtended` — extended state with PnL/equity/liq price flags | source: `sdk/typescript/dango/src/actions/perps/queries/getUserStateExtended.ts`
- `getPerpsOrdersByUser` — active perps orders for a user | source: `sdk/typescript/dango/src/actions/perps/queries/getOrdersByUser.ts`
- `getPerpsLiquidityDepth` — bid/ask depth buckets for a pair | source: `sdk/typescript/dango/src/actions/perps/queries/getLiquidityDepth.ts`
- `getPerpsPairParam` — `PerpsPairParam` for one pair | source: `sdk/typescript/dango/src/actions/perps/queries/getPairParam.ts`
- `getPerpsPairParams` — map of `PerpsPairParam` across pairs | source: `sdk/typescript/dango/src/actions/perps/queries/getPairParams.ts`
- `getPerpsParam` — global perps parameters | source: `sdk/typescript/dango/src/actions/perps/queries/getParam.ts`
- `getPerpsPairStats` — indexer 24h stats for a perps pair | source: `sdk/typescript/dango/src/actions/perps/queries/getPerpsPairStats.ts`
- `getAllPerpsPairStats` — indexer 24h stats for every perps pair | source: `sdk/typescript/dango/src/actions/perps/queries/getAllPerpsPairStats.ts`
- `getPerpsPairState` — `PerpsPairState` (OI, funding) for a pair | source: `sdk/typescript/dango/src/actions/perps/queries/getPerpsPairState.ts`
- `getPerpsState` — global runtime state (`PerpsState`) | source: `sdk/typescript/dango/src/actions/perps/queries/getPerpsState.ts`
- `getPerpsVaultState` — perps vault state with positions | source: `sdk/typescript/dango/src/actions/perps/queries/getVaultState.ts`
- `getVaultSnapshots` — historical vault equity/shares snapshots | source: `sdk/typescript/dango/src/actions/perps/queries/getVaultSnapshots.ts`
- `getFeeRateOverride` — per-user fee rate override | source: `sdk/typescript/dango/src/actions/perps/queries/getFeeRateOverride.ts`
- `queryPerpsCandles` — paginated perps candles | source: `sdk/typescript/dango/src/actions/perps/queries/perpsCandles.ts`
- `queryPerpsEvents` — paginated perps events (fills/liquidations/deleverages) | source: `sdk/typescript/dango/src/actions/perps/queries/perpsEvents.ts`

Mutations:
- `depositMargin` — deposit collateral into the perps account | source: `sdk/typescript/dango/src/actions/perps/mutations/depositMargin.ts`
- `withdrawMargin` — withdraw collateral from the perps account | source: `sdk/typescript/dango/src/actions/perps/mutations/withdrawMargin.ts`
- `submitPerpsOrder` — submit a market or limit perps order with optional TP/SL | source: `sdk/typescript/dango/src/actions/perps/mutations/submitOrder.ts`
- `cancelPerpsOrder` — cancel one/all perps orders | source: `sdk/typescript/dango/src/actions/perps/mutations/cancelOrder.ts`
- `submitConditionalOrder` — submit one trigger-based conditional order | source: `sdk/typescript/dango/src/actions/perps/mutations/submitConditionalOrder.ts`
- `submitConditionalOrders` — submit a batch of conditional orders | source: `sdk/typescript/dango/src/actions/perps/mutations/submitConditionalOrders.ts`
- `cancelConditionalOrder` — cancel conditional order(s) by request shape | source: `sdk/typescript/dango/src/actions/perps/mutations/cancelConditionalOrder.ts`
- `setReferral` — set a referrer/referee mapping | source: `sdk/typescript/dango/src/actions/perps/mutations/setReferral.ts`
- `setFeeShareRatio` — set the referrer fee share ratio | source: `sdk/typescript/dango/src/actions/perps/mutations/setFeeShareRatio.ts`
- `vaultAddLiquidity` — deposit into the perps vault | source: `sdk/typescript/dango/src/actions/perps/mutations/vaultAddLiquidity.ts`
- `vaultRemoveLiquidity` — burn vault shares for a withdrawal | source: `sdk/typescript/dango/src/actions/perps/mutations/vaultRemoveLiquidity.ts`

##### Domain: gateway
Queries:
- `getWithdrawalFee` — withdrawal fee for a denom + remote chain | source: `sdk/typescript/dango/src/actions/gateway/queries/getWithdrawalFee.ts`

Mutations:
- `transferRemote` — send funds to a remote chain via the gateway contract | source: `sdk/typescript/dango/src/actions/gateway/mutations/transferRemote.ts`

##### Domain: indexer
Queries:
- `queryIndexer` — generic typed GraphQL request against the indexer | source: `sdk/typescript/dango/src/actions/indexer/queryIndexer.ts`
- `queryBlock` — fetch a block (by height or latest) with transactions and outcomes | source: `sdk/typescript/dango/src/actions/indexer/queryBlock.ts`
- `searchTxs` — paginated transaction search by hash and/or sender | source: `sdk/typescript/dango/src/actions/indexer/searchTxs.ts`

Subscriptions:
- `accountSubscription` — account creation events for a user index | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/account.ts`
- `blockSubscription` — new finalized blocks (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/block.ts`
- `candlesSubscription` — live candles for a pair + interval (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/candles.ts`
- `eventsSubscription` — filtered live events stream (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/events.ts`
- `eventsByAddressesSubscription` — live events for a set of addresses (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/eventsByAddresses.ts`
- `transferSubscription` — live transfer events for a username (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/transfer.ts`
- `tradesSubscription` — live spot trades for a pair (WS + HTTP fallback) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/trades.ts`
- `perpsCandlesSubscription` — live perps candles (WS-only) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/perpsCandles.ts`
- `perpsTradesSubscription` — live perps trades for a pair (WS + HTTP fallback) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/perpsTrades.ts`
- `queryAppSubscription` — live result of a `queryApp` request (WS + HTTP fallback) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/queryApp.ts`
- `allPairStatsSubscription` — live 24h stats for all spot pairs (WS + HTTP fallback) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/allPairStats.ts`
- `allPerpsPairStatsSubscription` — live 24h stats for all perps pairs (WS + HTTP fallback) | source: `sdk/typescript/dango/src/actions/indexer/subscriptions/allPerpsPairStats.ts`

##### Domain: oracle
Queries:
- `getPrices` — paginated `Record<Denom, Price>` from the oracle contract | source: `sdk/typescript/dango/src/actions/oracle/queries/getPrices.ts`

##### Domain: hyperlane (top-level `./hyperlane` entry; not part of `actions`)
- `Addr32` / `toAddr32` — 32-byte Hyperlane address encoding helpers | source: `sdk/typescript/dango/src/hyperlane/addr32.ts`
- `Message`, `MAILBOX_VERSION`, `HYPERLANE_DOMAIN_KEY` — Hyperlane mailbox message encoder + constants | source: `sdk/typescript/dango/src/hyperlane/mailbox.ts`
- `IncrementalMerkleTree` — Hyperlane outbox Merkle tree implementation | source: `sdk/typescript/dango/src/hyperlane/merkletree.ts`
- `Metadata` — multisig ISM metadata encoder | source: `sdk/typescript/dango/src/hyperlane/multisig.ts`
- `TokenMessage` — warp-route token message encoder | source: `sdk/typescript/dango/src/hyperlane/warp.ts`
- `mockValidatorSet`, `mockValidatorSign` — test helpers for mocking a validator set | source: `sdk/typescript/dango/src/hyperlane/mock.ts`
- `ERC20_ABI`, `HYPERLANE_ROUTER_ABI`, `INFURA_URLS` — Ethereum ABIs and Infura RPC URLs used when relaying | source: `sdk/typescript/dango/src/hyperlane/abis.ts`

#### Types (exported from the entry point)
- `PublicClient` — `Client<undefined, PublicActions>` | source: `sdk/typescript/dango/src/clients/publicClient.ts`
- `SignerClient` — `Client<Signer, PublicActions & SignerActions>` | source: `sdk/typescript/dango/src/clients/signerClient.ts`
- `PublicActions` — composed query action surface | source: `sdk/typescript/dango/src/actions/publicActions.ts`
- `SignerActions` — composed mutation action surface | source: `sdk/typescript/dango/src/actions/signerActions.ts`

#### Errors
- `BaseError` — base class with `shortMessage`, `details`, `metaMessages` | source: `sdk/typescript/dango/src/errors/base.ts`
- `HttpRequestError` — thrown when a GraphQL HTTP request fails (includes status, url, body) | source: `sdk/typescript/dango/src/errors/request.ts`
- `TimeoutError` — thrown when a request exceeds its timeout | source: `sdk/typescript/dango/src/errors/timeout.ts`
- `UrlRequiredError` — thrown when transport is invoked without a URL | source: `sdk/typescript/dango/src/errors/transports.ts`

> Note: `BaseError`, `TimeoutError`, `HttpRequestError`, `UrlRequiredError` are exported via `#errors/*` internal paths but the entry point does NOT re-export them. They are publicly thrown but consumers cannot import them by name from `@left-curve/sdk`. See Verification TODOs.

### @left-curve/crypto
Cryptographic primitives — hashes, key pairs (secp256k1 / ed25519), and WebAuthn.

#### Clients
*(none)*

#### Actions
*(no domain split — flat utility surface)*
- `sha256` / `Sha256` — SHA-256 hash function + streaming class | source: `sdk/typescript/crypto/src/sha.ts`
- `sha512` / `Sha512` — SHA-512 hash function + streaming class | source: `sdk/typescript/crypto/src/sha.ts`
- `keccak256` / `Keccak256` — Keccak-256 hash function + streaming class | source: `sdk/typescript/crypto/src/sha.ts`
- `ripemd160` / `Ripemd160` — RIPEMD-160 hash function + streaming class | source: `sdk/typescript/crypto/src/ripemd.ts`
- `Secp256k1` — secp256k1 KeyPair class (`makeKeyPair`, `fromMnemonic`, sign/verify) | source: `sdk/typescript/crypto/src/keys/secp256k1.ts`
- `Ed25519` — ed25519 KeyPair class (`makeKeyPair`, `fromMnemonic`, sign/verify) | source: `sdk/typescript/crypto/src/keys/ed25519.ts`
- `secp256k1RecoverPubKey` — recover the public key from a signature + hash | source: `sdk/typescript/crypto/src/keys/secp256k1.ts`
- `secp256k1CompressPubKey` — toggle a secp256k1 pubkey between compressed/uncompressed | source: `sdk/typescript/crypto/src/keys/secp256k1.ts`
- `secp256k1VerifySignature` — verify a secp256k1 signature against a hash + pubkey | source: `sdk/typescript/crypto/src/keys/secp256k1.ts`
- `ed25519VerifySignature` — verify an ed25519 signature | source: `sdk/typescript/crypto/src/keys/ed25519.ts`
- `ethHashMessage` — EIP-191 "Ethereum Signed Message" hash | source: `sdk/typescript/crypto/src/signature/ethHashMessage.ts`
- `createWebAuthnCredential` — create a P-256 passkey credential (browser WebAuthn) | source: `sdk/typescript/crypto/src/webauthn/create.ts`
- `getCredentialCreationOptions` — build the `PublicKeyCredentialCreationOptions` payload | source: `sdk/typescript/crypto/src/webauthn/create.ts`
- `createChallenge` — generate a 16-byte random challenge | source: `sdk/typescript/crypto/src/webauthn/create.ts`
- `requestWebAuthnSignature` — request a signature from an existing passkey | source: `sdk/typescript/crypto/src/webauthn/signature.ts`
- `getCredentialSignRequestOptions` — build the `PublicKeyCredentialRequestOptions` payload | source: `sdk/typescript/crypto/src/webauthn/signature.ts`
- `parseAsn1Signature` — split an ASN.1 ECDSA signature into raw `r,s` bytes | source: `sdk/typescript/crypto/src/webauthn/signature.ts`
- `verifyWebAuthnSignature` — verify a P-256 WebAuthn signature against a public key | source: `sdk/typescript/crypto/src/webauthn/verify.ts`

#### Types
- `KeyPair` — common key-pair interface (`getPublicKey`, `createSignature`, `verifySignature`) | source: `sdk/typescript/crypto/src/keys/keypair.ts`
- `CredentialAttestion`, `CreateCredentialParameters`, `CredentialOptionParameters` — WebAuthn creation types | source: `sdk/typescript/crypto/src/webauthn/create.ts`
- `CredentialAssertion`, `CredentialRequestOptionParameters`, `SignParameters`, `WebAuthnData` — WebAuthn signing types | source: `sdk/typescript/crypto/src/webauthn/signature.ts`
- `VerifyParameters` — WebAuthn verification params | source: `sdk/typescript/crypto/src/webauthn/verify.ts`
- `EthPersonalMessage` — input type for `ethHashMessage` | source: `sdk/typescript/crypto/src/signature/ethHashMessage.ts`

#### Errors
*(none — module throws plain `Error`)*

### @left-curve/encoding
Hex / base64 / UTF-8 / endian / JSON encoding and binary (de)serialization helpers.

#### Clients
*(none)*

#### Actions
- `encodeHex` / `decodeHex` — lowercase hex encode/decode (with optional `0x` prefix) | source: `sdk/typescript/encoding/src/hex.ts`
- `isHex` — type-guard: is a value a hex string | source: `sdk/typescript/encoding/src/hex.ts`
- `hexToBigInt` — parse a hex string as `bigint` | source: `sdk/typescript/encoding/src/hex.ts`
- `encodeBase64` / `decodeBase64` — standard base64 codec | source: `sdk/typescript/encoding/src/base64.ts`
- `encodeBase64Url` / `decodeBase64Url` — URL-safe base64 codec | source: `sdk/typescript/encoding/src/base64.ts`
- `base64ToBase64Url` / `base64UrlToBase64` — base64 <-> base64url string conversion | source: `sdk/typescript/encoding/src/base64.ts`
- `encodeUtf8` / `decodeUtf8` — UTF-8 codec (lossy mode optional) | source: `sdk/typescript/encoding/src/utf8.ts`
- `encodeEndian32` / `decodeEndian32` — 32-bit big/little-endian codec | source: `sdk/typescript/encoding/src/endian32.ts`
- `encodeUint` — encode a numeric string as a fixed-width big-endian integer | source: `sdk/typescript/encoding/src/uint.ts`
- `serialize` / `deserialize` — payload <-> bytes (snake_case JSON + UTF-8) used for signing/hashing | source: `sdk/typescript/encoding/src/binary.ts`
- `serializeJson` / `deserializeJson` — superjson-based JSON codec that preserves `Uint8Array` | source: `sdk/typescript/encoding/src/json.ts`
- `sortedJsonStringify` / `sortedObject` — deterministic JSON output | source: `sdk/typescript/encoding/src/json.ts`
- `snakeCaseJsonSerialization` / `camelCaseJsonDeserialization` — recursive key-case transforms | source: `sdk/typescript/encoding/src/json.ts`

#### Types
*(none re-exported from entry point)*

#### Errors
*(none — module throws plain `Error`)*

### @left-curve/types
TypeScript-only type definitions used across the SDK. No runtime exports except for a handful of `as const` enum-like maps.

#### Clients
*(none)*

#### Actions
*(none)*

#### Runtime constants (exported alongside types)
- `Direction` — `{ Buy: "bid", Sell: "ask" }` const map | source: `sdk/typescript/types/src/dex.ts`
- `OrderType` — `{ Limit, Market }` const map | source: `sdk/typescript/types/src/dex.ts`
- `TimeInForceOption` — `{ GoodTilCanceled, ImmediateOrCancel }` const map | source: `sdk/typescript/types/src/dex.ts`
- `CandleInterval` — interval label const map | source: `sdk/typescript/types/src/dex.ts`
- `PoolType` — `{ Xyk, Concentrated }` const map | source: `sdk/typescript/types/src/pool.ts`
- `UserState` — `{ Active, Inactive, Frozen }` const map | source: `sdk/typescript/types/src/account.ts`
- `KeyTag` — `{ secp256r1: 0, secp256k1: 1, ethereum: 2 }` discriminant map | source: `sdk/typescript/types/src/key.ts`

#### Types
Address / coins / chain:
- `Address` — `` `0x${string}` `` branded string | source: `sdk/typescript/types/src/address.ts`
- `Denom`, `Coin`, `Coins`, `Funds` — coin denomination + amount types | source: `sdk/typescript/types/src/coins.ts`
- `Chain`, `ChainId` — chain config | source: `sdk/typescript/types/src/chain.ts`

Encoding / common:
- `Json`, `JsonValue`, `JsonString`, `Hex`, `Base64`, `Binary`, `Encoder`, `DateTime` | source: `sdk/typescript/types/src/encoding.ts`
- `UID` | source: `sdk/typescript/types/src/common.ts`

Client / transport / signer:
- `Client`, `ClientConfig`, `ClientExtend` | source: `sdk/typescript/types/src/client.ts`
- `PublicClientConfig`, `SignerClientConfig` | source: `sdk/typescript/types/src/clients.ts`
- `Transport`, `RequestFn`, `SubscribeFn`, `SubscriptionCallbacks`, `RequestOptions` | source: `sdk/typescript/types/src/transports.ts`
- `Signer` | source: `sdk/typescript/types/src/signer.ts`

Accounts / keys / sessions:
- `User`, `Username`, `UserIndexOrName`, `UserStatus`, `Account`, `AccountDetails`, `AccountIndex`, `AccountInfo` | source: `sdk/typescript/types/src/account.ts`
- `Key`, `KeyHash`, `PublicKey` | source: `sdk/typescript/types/src/key.ts`
- `Credential`, `StandardCredential`, `SessionCredential` | source: `sdk/typescript/types/src/credential.ts`
- `SigningSession`, `SigningSessionInfo`, `SessionResponse` | source: `sdk/typescript/types/src/session.ts`

Signatures / typed data:
- `Signature`, `Secp256k1Signature`, `PasskeySignature`, `Eip712Signature`, `RawSignature`, `SignDoc`, `ArbitraryDoc`, `SignatureOutcome`, `ArbitrarySignatureOutcome` | source: `sdk/typescript/types/src/signature.ts`
- `TypedData`, `TypedDataParameter`, `TypedDataProperty`, `ArbitraryTypedData`, `EIP712Domain`, `EIP712Message`, `EIP712Types`, `DomainType`, `MessageType`, `MetadataType`, `TxMessageType`, `SolidityTypes` | source: `sdk/typescript/types/src/typedData.ts`
- `Metadata` (tx metadata: username, chainId, nonce, expiry) | source: `sdk/typescript/types/src/metadata.ts`

App / queries / tx:
- `AppConfig`, `ChainConfig`, `ContractInfo`, `BlockInfo`, `Duration`, `Timestamp`, `Permission`, `EverybodyPermission`, `SomebodiesPermission`, `NobodyPermission` | source: `sdk/typescript/types/src/app.ts`
- `Code`, `CodeStatus` | source: `sdk/typescript/types/src/code.ts`
- `QueryRequest`, `QueryResponse`, `ChainConfigResponse`, `ChainStatusResponse`, `QueryContractRequest`, `QueryContractsRequest`, `QueryCodesRequest`, `QueryBalanceRequest`, `QueryBalancesRequest`, `QueryCodeRequest`, `QueryConfigRequest`, `QuerySupplyRequest`, `QuerySuppliesRequest`, `QueryStatusRequest`, `QueryAppConfigRequest`, `QueryAppConfigsRequest`, `QueryWasmRawRequest`, `QueryWasmSmartRequest`, `WasmRawResponse`, `WasmSmartResponse`, `CodeResponse`, `CodesResponse`, `ContractResponse`, `ContractsResponse`, `AppConfigResponse`, `StatusResponse` | source: `sdk/typescript/types/src/queries.ts`
- `SimulateRequest`, `SimulateResponse` | source: `sdk/typescript/types/src/simulate.ts`
- `Tx`, `UnsignedTx`, `TxParameters`, `Message`, `MsgConfigure`, `MsgExecute`, `MsgInstantiate`, `MsgMigrate`, `MsgStoreCode`, `MsgTransfer`, `GetTxMessage` | source: `sdk/typescript/types/src/tx.ts`
- `Proof`, `MembershipProof`, `NonMembershipProof`, `Node`, `InternalNode`, `LeafNode` | source: `sdk/typescript/types/src/proof.ts`

Events / indexer / cometbft:
- `IndexedEvent`, `EventStatus`, `CommitmentStatus`, `EventData`, `ContractEvent`, `TransferEvent`, `ExecuteEvent`, `OrderCreatedEvent`, `OrderCanceledEvent`, `OrderFilledEvent`, `EventFilter`, `EventFilterData`, `SubscriptionEvent` | source: `sdk/typescript/types/src/event.ts`
- `IndexedBlock`, `IndexedTransaction`, `IndexedMessage`, `IndexedTransactionType`, `IndexedTransferEvent`, `IndexedTrade`, `IndexedTradeSideType`, `IndexedAccountEvent`, `PerpsTrade`, `PerpsEvent`, `PerpsEventType`, `OrderFilledData`, `LiquidatedData`, `DeleveragedData` | source: `sdk/typescript/types/src/indexer.ts`
- `QueryAbciResponse`, `TxResponse`, `TxProof`, `TxData`, `TxEvent`, `TxEventAttribute`, `ProofOp` | source: `sdk/typescript/types/src/cometbft.ts`

DEX:
- `DexExecuteMsg`, `DexQueryMsg`, `GetDexExecuteMsg`, `GetDexQueryMsg`, `Directions`, `CoinPair`, `OrderResponse`, `OrdersByPairResponse`, `OrdersByUserResponse`, `PairId`, `PairSymbols`, `ReservesResponse`, `SwapRoute`, `PairParams`, `PairUpdate`, `CancelOrderRequest`, `CreateOrderRequest`, `PriceOption`, `AmountOption`, `OrderId`, `Candle`, `CandleIntervals`, `PerpsCandle`, `Trade`, `TimeInForceOptions`, `OrderTypes`, `RestingOrderBookState`, `LiquidityDepth`, `LiquidityDepthResponse`, `PairStats`, `PerpsPairStats` | source: `sdk/typescript/types/src/dex.ts`

Perps:
- `RateSchedule`, `PerpsUserState`, `PerpsUserStateExtended`, `PerpsPosition`, `PerpsPositionExtended`, `PerpsUnlock`, `PerpsOrderKind`, `PerpsTimeInForce`, `PerpsPairParam`, `PerpsPairState`, `PerpsParam`, `PerpsState`, `PerpsOrderResponse`, `PerpsOrderByUserItem`, `PerpsOrdersByUserResponse`, `PerpsLiquidityDepth`, `PerpsLiquidityDepthResponse`, `PerpsCancelOrderRequest`, `PerpsCancelConditionalOrderRequest`, `PerpsQueryMsg`, `GetPerpsQueryMsg`, `FeeRateOverride`, `PerpsVaultState`, `TriggerDirection`, `ChildOrder`, `ConditionalOrder`, `VaultSnapshot` | source: `sdk/typescript/types/src/perps.ts`

Pool:
- `Pool`, `PoolId`, `PoolInfo`, `PoolParams`, `PoolTypes`, `XykParams`, `XykPool`, `ConcentratedParams`, `ConcentratedPool`, `FeeRate` | source: `sdk/typescript/types/src/pool.ts`

Hyperlane:
- `Addr32`, `Domain`, `Remote`, `WarpRemote`, `BitcoinRemote`, `MailBoxConfig`, `HyperlaneConfig` | source: `sdk/typescript/types/src/hyperlane.ts`

Oracle:
- `Price` | source: `sdk/typescript/types/src/oracle.ts`

GraphQL:
- `GraphqlPagination`, `GraphqlQueryResult`, `GraphqlClient`, `GraphqlClientOptions`, `GraphqlOperation`, `GraphQLClientResponse`, `HttpRequestParameters` | source: `sdk/typescript/types/src/graphql.ts`

WebRTC:
- `DataChannelConfig`, `DataChannelMessage` | source: `sdk/typescript/types/src/webrtrc.ts`

Utility / generic:
- `Prettify`, `OneOf`, `OneRequired`, `RequiredBy`, `ExactPartial`, `ExactRequired`, `RemoveUndefined`, `StrictOmit`, `UnionStrictOmit`, `MaybePromise`, `Failure`, `Success`, `Result`, `Option`, `AllLeafKeys`, `KeyOfUnion`, `ExtractFromUnion`, `NestedOmit`, `WithId`, `Flatten`, `Range`, `ValueFunction`, `ValueOrFunction`, `Require`, `StdResult`, `NonNullableProperties`, `NonNullablePropertiesBy`, `WithPrice`, `WithAmount`, `WithDecimals` | source: `sdk/typescript/types/src/utils.ts`

#### Errors
*(none)*

### @left-curve/utils
General-purpose utilities: `Decimal` math, formatters, async helpers, DEX/vault calculations, and the subscription/polling helpers used by the SDK.

#### Clients
*(none)*

#### Actions
Strings:
- `camelToSnake`, `snakeToCamel`, `capitalize`, `camelToTitleCase`, `truncateAddress` | source: `sdk/typescript/utils/src/strings.ts`

Mappers:
- `recursiveTransform`, `mayTransform`, `sortObject`, `invertObject`, `plainObject` | source: `sdk/typescript/utils/src/mappers.ts`

Assertions:
- `assertSet`, `assertBoolean`, `assertString`, `assertNumber`, `assertArray`, `assertObject`, `assertNotEmpty`, `assertDeepEqual` | source: `sdk/typescript/utils/src/asserts.ts`

Promises / async:
- `wait`, `withRetry`, `withTimeout`, `withResolvers` | source: `sdk/typescript/utils/src/promises.ts`

Scheduling / polling:
- `createBatchScheduler` — coalesce many calls into one batched fn call | source: `sdk/typescript/utils/src/scheduler.ts`
- `batchPoller` — singleton interval coordinator for HTTP polls | source: `sdk/typescript/utils/src/batchPoller.ts`
- `debounce` — debounce a function | source: `sdk/typescript/utils/src/frequency.ts`
- `createSubscription` — WS + HTTP-fallback subscription helper used by all `*Subscription` actions | source: `sdk/typescript/utils/src/createSubscription.ts`

Misc:
- `uid` — short random id (viem-style) | source: `sdk/typescript/utils/src/uid.ts`
- `tryCatch` — wrap a thrown error into a `Result` | source: `sdk/typescript/utils/src/tryCatch.ts`
- `randomBetween` — inclusive random integer | source: `sdk/typescript/utils/src/numbers.ts`

Numbers / formatting:
- `Decimal` (default export) — arbitrary-precision decimal wrapping `big.js`; the canonical financial-math type | source: `sdk/typescript/utils/src/decimal.ts`
- `formatNumber`, `formatDisplayNumber`, `formatDisplayString`, `bucketSizeToFractionDigits`, `truncateDec`, `formatUnits`, `parseUnits` | source: `sdk/typescript/utils/src/formatters.ts`

Browser detection:
- `getNavigatorOS`, `getRootDomain`, `isMobileOrTable` | source: `sdk/typescript/utils/src/browser.ts`

Typed data composition:
- `getCoinsTypedData`, `composeTxTypedData`, `composeArbitraryTypedData` | source: `sdk/typescript/utils/src/typedData.ts`

DEX helpers:
- `calculateTradeSize`, `calculateFees`, `calculatePrice`, `formatOrderId`, `adjustPrice`, `resolveRateSchedule` | source: `sdk/typescript/utils/src/dex.ts`

Vault helpers:
- `sharesToUsd`, `usdToShares`, `computeVaultApy` | source: `sdk/typescript/utils/src/vault.ts`

#### Types
- `FormatNumberOptions`, `DisplayPart` — types for `formatNumber` | source: `sdk/typescript/utils/src/formatters.ts`
- `SubscriptionOptions`, `TransportMode` — types for `createSubscription` | source: `sdk/typescript/utils/src/createSubscription.ts`

#### Errors
*(none — module throws plain `Error`)*

### @left-curve/config
Shared TypeScript / Biome / tsup config presets consumed via `files: ["ts", "biome", "tsup"]` — no runtime exports.

#### Clients / Actions / Types / Errors
*(none — config-only package)*

## Cross-package re-exports

`@left-curve/sdk` re-exports the following from sibling packages so consumers can import them from a single entry point:

From `@left-curve/types`:
- `Address`, `Coin`, `Coins`, `Chain`, `Denom`, `KeyHash`, `Account`
- `PublicClientConfig`, `SignerClientConfig`
- `RateSchedule`, `PerpsUserState`, `PerpsUserStateExtended`, `PerpsPosition`, `PerpsPositionExtended`, `PerpsUnlock`, `PerpsOrderKind`, `PerpsTimeInForce`, `PerpsPairParam`, `PerpsPairState`, `PerpsParam`, `PerpsState`, `PerpsOrderResponse`, `PerpsOrderByUserItem`, `PerpsOrdersByUserResponse`, `PerpsLiquidityDepth`, `PerpsLiquidityDepthResponse`, `PerpsCancelOrderRequest`, `PerpsCancelConditionalOrderRequest`, `PerpsQueryMsg`, `GetPerpsQueryMsg`, `FeeRateOverride`, `PerpsVaultState`, `TriggerDirection`, `ChildOrder`, `ConditionalOrder`, `VaultSnapshot`
- `Direction`, `OrderType`, `TimeInForceOption` (const maps)

From `@left-curve/utils`:
- `formatUnits`, `parseUnits`

From `@left-curve/crypto`:
- `Secp256k1`

The `@left-curve/sdk/actions` subpath additionally re-exports two perps types directly:
- `FeeRateOverride`, `VaultSnapshot` (from `@left-curve/types`) — re-exported by `sdk/typescript/dango/src/actions/perps/index.ts`

## Excluded items (do not document)

- `sdk/typescript/dango/src/http/graphqlClient.ts` — internal HTTP helper used by `createTransport`; not re-exported.
- `HttpRequestErrorType`, `TimeoutErrorType` (in `sdk/typescript/dango/src/errors/request.ts`, `timeout.ts`) — internal helper type aliases (`X & { name: "X" }`), not re-exported.
- `sdk/typescript/dango/src/account/accountInfo.ts:ToAccountParameters` — exported with `toAccount` but only the function is re-exported from the entry. Drafters can document it as part of `toAccount`.
- `sdk/typescript/crypto/src/keys/keypair.ts` — `KeyPair` is re-exported, but the rest of the module is type-only.
- `sdk/typescript/utils/src/decimal.ts:BigSource`, `DecimalConstructor` — internal helper types for the `Decimal` class. Document as part of `Decimal`.
- `sdk/typescript/utils/src/scheduler.ts:CreateBatchSchedulerArguments`, `CreateBatchSchedulerReturnType` — internal types not re-exported.
- `sdk/typescript/utils/src/createSubscription.ts:TransportMode` — exported but only consumed internally by `createSubscription`; document as part of `createSubscription`.
- `sdk/typescript/dango/src/hyperlane/abis.ts:INFURA_URLS` — hard-coded Infura URLs (with embedded keys) used only by Hyperlane mock helpers. Reason: experimental / questionable to advertise as public API; flagging for the maintainers.
- `sdk/typescript/dango/src/hyperlane/mock.ts:mockValidatorSet`, `mockValidatorSign` — explicitly labeled `mock` test helpers. Document only under a "Testing" concept page, not as production API.
- `sdk/typescript/types/src/webrtrc.ts` — `webrtrc` is misspelled (should be `webrtc`); the only types `DataChannelConfig`, `DataChannelMessage` are exported but appear unused by anything else in this repo. Suspected dead code; flag to maintainers.
- `sdk/typescript/types/src/dex.ts:CurveInvariant`, `CurveInvariants` — exported in source but NOT re-exported from `@left-curve/types/index.ts`. Reason: internal.

## Verification TODOs (drafters must confirm before writing)

- `queryApp` / `queryWasmSmart` / `queryStatus` return types — `QueryResponse` and `WasmSmartResponse` are discriminated unions; drafter must enumerate the per-action narrowed return type and document the runtime error thrown when narrowing fails.
- `simulate` return — function rescales `gasUsed` by a default `scale = 1.3`. Drafter must call out this multiplier in the docs (it affects gas-limit calculation downstream).
- `getBalance` — returns a `number` (parsed via `parseInt`). This will overflow for amounts above 2^53. Verify with maintainers whether this is intentional or a bug; if intentional, document the precision limitation. (Compare with all other balance-shaped fields in the SDK which use `string`.)
- `getAppConfig` — uses a module-level cache (`let config`). The `height` parameter is silently ignored after the first successful call. Drafter must document the cache behavior or flag to maintainers.
- Errors (`BaseError`, `HttpRequestError`, `TimeoutError`, `UrlRequiredError`) — exported via `#errors/*` internal alias paths but the package entry (`src/index.ts`) does NOT re-export them. Drafter must either (a) propose adding them to the entry barrel, or (b) document that errors must be type-narrowed by `name` / `instanceof Error`. Confirm with maintainers before writing the Errors section.
- `Decimal` — re-exported as `default export` from `@left-curve/utils/src/decimal.ts` and surfaced as `{ default as Decimal }`. Drafter must verify import idiom: `import { Decimal } from "@left-curve/utils"` works, not `import Decimal from "@left-curve/utils/decimal"`.
- Action functions vs builder objects: the entry barrel `sdk/typescript/dango/src/actions/index.ts` re-exports both raw functions (`getBalance`) and builder objects (`appQueryActions`). The Action template likely wants one primary form. Verify with style guide intent before writing — recommend documenting the raw function with a tab showing client-extended usage.
- `gateway*` actions are namespaced (`client.gateway.getWithdrawalFee(...)` not `client.getWithdrawalFee(...)`). Other domains are flat. Drafter must show the correct namespacing in examples for gateway actions only.
- `createTransport` — second-arg config (`GraphqlTransportConfig`) has many fields. Drafter must enumerate every option (batching, WS retry, `lazy`, `disableWs`, `polling`, fetch hooks) — this is the primary public surface of the package.
- `registerAccount` action — its `registerAccountMutationActions` wrapper accepts `txArgs?: TxParameters` as a second function argument (a positional, not in `parameters`). Documented signature must match. Verify the action builder version applies `txArgs` correctly.
- `submitConditionalOrders` — throws if `orders.length === 0`. Document the precondition.
- Type re-exports: `sdk/typescript/dango/src/actions/perps/index.ts` re-exports `FeeRateOverride` and `VaultSnapshot` from `@left-curve/types`. This means an end user can import them from `@left-curve/sdk/actions` as well as `@left-curve/sdk` and `@left-curve/types`. Pick one canonical import path in docs.
- `IndexerActions` has both queries (`queryBlock`, `searchTxs`) and subscription helpers (`*Subscription`) on the same builder. Sitemap groups Actions by domain, but the docs need to distinguish subscriptions vs one-shot calls — clarify with the style guide.
- `createSessionSigner` signs against `payload.message` for `signArbitrary` (PrivateKeySigner signs the full `payload`). Documented behavior must reflect this divergence between signer implementations.
