# Hyperlane agents (relayer + validator)

This role deploys the Hyperlane **relayer** and **validator** agents (image
`ghcr.io/left-curve/left-curve/hyperlane-agents`) as standalone docker-compose
stacks, one instance per host. They are NOT CometBFT validators — they are the
interchain-messaging agents that move Warp-route messages between Dango and the
EVM chains.

- **Validator** — watches the Dango mailbox (`originchainname: dango[testnet]`)
  and signs merkle checkpoints. Covers the **withdraw** direction (Dango → EVM).
- **Relayer** — relays messages among `relaychains` and submits the delivery
  (`process`) tx on the destination. Needs a **funded** signer on each EVM chain.

## Config layering

The agent merges several config sources (later overrides earlier):

1. **Bundled registry** baked into the image — `rust/main/config/{testnet,mainnet}_config.json`
   in the hyperlane-monorepo. Provides each chain's `domainId`, `mailbox`,
   `interchainGasPaymaster`, `validatorAnnounce`, `merkleTreeHook` and public
   `rpcUrls`. Most public chains (sepolia, arbitrum, arbitrumsepolia, ...) are
   already here.
2. **`files/config.json`** — shared by all agents, mounted and loaded via
   `CONFIG_FILES`. Per-chain overrides: `index.from` / `index.chunk` and
   `signer.type`.
3. **`files/relayer-<net>-config.json`** — `relaychains` (which chains the
   relayer operates on) + `whitelist` (which routes it will deliver) + optional
   per-chain flags (e.g. `ignoreReorgReports`).
4. **`templates/relayer-<net>.env.j2`** — per-chain signer keys and optional
   custom RPC URLs, rendered from the vault into the on-host `.env` (the `.env`
   itself is never committed).
5. **Vault** (`group_vars/hyperlane/vault.yml`, edited via
   `just edit-hyperlane-secrets`) — `relayer.<net>.<chain>_signer_key` and
   `relayer.<net>.<chain>_rpc_url`.

## Adding a new EVM chain to the relayer

Worked example: `arbitrum` (Arbitrum One, domain `42161`). Dango domains:
`88888888` mainnet, `88888887` testnet.

1. **Registry** — confirm the chain exists in the bundled `*_config.json`
   (name + `domainId` + `mailbox`). If not, it must be added to the agent image
   first.
2. **`files/config.json`** — add the chain:
   ```json
   "arbitrum": { "index": { "chunk": 50, "from": <DEPLOY_BLOCK> }, "signer": { "type": "hexKey" } }
   ```
   See the `index.from` gotcha below.
3. **`files/relayer-<net>-config.json`** — add to `relaychains` and the
   `whitelist` (both directions, dango ↔ new chain):
   ```json
   "relaychains": "dango,ethereum,arbitrum",
   "whitelist": [
     ...,
     { "originDomain": "42161",    "destinationDomain": "88888888", "senderAddress": "*", "recipientAddress": "*" },
     { "originDomain": "88888888", "destinationDomain": "42161",    "senderAddress": "*", "recipientAddress": "*" }
   ]
   ```
   The whitelist is an **allowlist**: only listed routes are delivered, so it
   implicitly blocks EVM↔EVM (e.g. ethereum↔arbitrum). Omitting it relays every
   pair among `relaychains`.
4. **`templates/relayer-<net>.env.j2`** — add the signer (+ optional custom RPC):
   ```jinja
   # Arbitrum chain signer
   HYP_CHAINS_ARBITRUM_SIGNER_KEY="{{ r.arbitrum_signer_key }}"

   {% if r.arbitrum_rpc_url is defined %}
   HYP_CHAINS_ARBITRUM_CUSTOMRPCURLS="{{ r.arbitrum_rpc_url }}"
   HYP_CHAINS_ARBITRUM_RPCCONSENSUSTYPE="single"
   {% endif %}
   ```
   The signer line is non-conditional, so the playbook fails fast if the vault
   secret is missing.
5. **Vault** (`just edit-hyperlane-secrets`), under `relayer.<net>`:
   - `arbitrum_signer_key` — private key of an EOA **funded with gas on that
     chain** (the relayer pays for `process` delivery). Can reuse the ethereum
     key (same address) but fund it on the new chain.
   - `arbitrum_rpc_url` — a **server-usable** RPC (see RPC gotcha).

## Gotchas — read before adding a chain

### 1. db_loader cold-start — you MUST deploy the relayer twice

When you add a **high-nonce origin chain**, the relayer's **message processor
(`db_loader`)** cold-starts broken. This is *separate* from the indexing sync
cursor, which works fine (the message is indexed and in the DB).

`ForwardBackwardIterator::new` calls `retrieve_highest_seen_message_nonce()`; on
the empty per-chain DB it returns `None`, so the "high" iterator starts at nonce
`0` and **freezes** there — it waits for nonce 0, which is never indexed because
indexing starts at a high block (e.g. arbitrumsepolia block `277880000` ≈ nonce
`486028`). Result: **no message for that chain is ever delivered** — the
already-dispatched ones AND all new ones. Sending a "test deposit" before the
fix freezes too and locks funds.

- **Symptom gauge:** `hyperlane_last_known_message_nonce{phase="db_loader_loop", origin="<chain>"} = 0`
  (would equal the real nonce if positioned correctly).

**Fix = deploy the relayer a SECOND time:**

1. First deploy — starts indexing the new chain; db_loader cold-starts at 0.
2. Wait until the chain has caught up indexing (cursor at head).
3. Second deploy (restart) — `highest_seen_message_nonce` is now persisted to
   RocksDB (`hyperlane_db.rs:676`), so on restart the high iterator starts from
   the real nonce, finds the stuck messages `Processable`, and delivers them.

**Do NOT** wipe `hyperlane_db` to "reset" it — it re-empties the per-chain DB
and re-triggers the exact same cold-start. (There is no clean/reset playbook;
both `deploy-hyperlane` and `stop-hyperlane` preserve the volume on purpose.)

### 2. RPC URL must have the right path and be server-usable

- Infura needs the `/v3/<KEY>` path. Without it → HTTP 404 with an **empty
  body** → the relayer logs `SerdeJson error ... "EOF while parsing a value",
  text: ""` and the chain fails to build (`Hit max requests`, `MissingConfiguration`).
- **Frontend Infura keys are usually origin/JWT-restricted**: they work in the
  browser but return HTTP 403 `"rejected due to project ID settings"`
  server-side. Use a key that works from a server.
- With `RPCCONSENSUSTYPE=single` there is **no fallback** — one bad URL kills
  the chain. Test the endpoint first: a POST `eth_chainId` must return the
  chain id (e.g. `0xa4b1` for Arbitrum One).

### 3. index.from

Set it `<=` the Warp-route deploy block (otherwise you miss its messages), but
**as recent as possible**. The registry defaults are ancient (mainnet arbitrum
= `143649797` vs head `~474M`) → that would backfill hundreds of millions of
blocks. On a shared public mailbox the relayer indexes ALL traffic in the range
but only *processes* whitelisted routes.

### 4. Deposit direction needs validators

For `<chain> → dango` (deposits) the **Dango ISM** must trust validator(s)
watching that chain's mailbox, and they must be up — otherwise messages stick on
`Could not fetch metadata: Unable to reach quorum` (we saw a ~47 min stall when
the Sepolia validator was down). Our deploy only runs **dango-origin**
validators (withdraw direction); EVM-origin validators come from elsewhere.

## Deploy & verify

```bash
cd ~/github/left-curve/deploy
# add a chain → deploy, wait for indexing to catch up, then deploy AGAIN (cold-start)
just deploy-hyperlane <net> relayer <host>
```

Relayer hosts: **testnet** = hetzner4 (`100.109.200.70`), **mainnet** = hetzner3
(`100.76.197.30`).

Verify via Prometheus (`ovh3:9091`), relayer metrics port **9501** mainnet /
**9511** testnet:

| Metric | Expectation |
|---|---|
| `hyperlane_critical_error{chain="<chain>"}` | `0` — RPC / chain build OK |
| `hyperlane_cursor_current_block{chain="<chain>"}` | climbing to chain head |
| `hyperlane_last_known_message_nonce{phase="db_loader_loop", origin="<chain>"}` | **> 0** after the second deploy (cold-start cleared — the key check) |
| `hyperlane_last_known_message_nonce{phase="message_processed", origin="<chain>"}` | moves when a real route message is relayed |

Logs (Loki on `ovh3:3100`): `{container_name="<net>-relayer"}`. A healthy chain
logs `Found log(s) in index range`; a stuck delivery logs
`Could not fetch metadata: Unable to reach quorum`.
