# Node Migration

Step-by-step runbook for moving an existing mainnet validator role from one server to another. The **source** server is currently running the chain (validating, hosting traefik/cloudflared/postgres/clickhouse, running its hyperlane validator instance, etc.). The **target** server has finished [`NEW_SERVER_SETUP.md`](NEW_SERVER_SETUP.md) — i.e. it's on the tailnet, in the WireGuard mesh, has the deploy user and Docker, and is running node-exporter + promtail, but has no application services yet.

The objective is to move all chain data, indexer state, and the validator identity from source to target with minimal disruption to mainnet.

## Prerequisites

- **Source's tailscale IP**, referenced as `<source IP>` below.
- **Target's tailscale IP**, referenced as `<target IP>`.
- **Source and target hostnames** (e.g. `inter1`, `hetzner5`).
- **Vault access** and keys in agent (`just add-deploy-key && just add-debian-key`).
- **Disk space on target**: at least 1.5× the sum of source's `~/mainnet/`, `~/psql/data/`, and `~/clickhouse/data/`. Run `ssh deploy@<source IP> 'du -sh ~/mainnet ~/psql/data ~/clickhouse/data'` to check.
- **Time window**: budget ~60–90 minutes. Source is offline for the duration of steps 1, 5–10. The chain runs on 3/4 quorum during that window — every block is slower because source's proposer slot times out, and a single additional validator failure halts consensus.

All commands assume `deploy/` is your working directory. Ansible runs through `uv`.

## Step 1. Stop all services on source

Stop dango + cometbft via the existing playbook (`just stop-mainnet`'s `--limit` is hardcoded for the full validator set, so call ansible directly to scope to source). Then stop postgres and clickhouse — no playbook covers stop for those.

```bash
uv run ansible-playbook stop-services.yml -e dango_network=mainnet --limit <source IP>
ssh deploy@<source IP> 'cd ~/psql && docker compose stop'
ssh deploy@<source IP> 'cd ~/clickhouse && docker compose stop'
```

> [!NOTE]
> Source remains in inventory and Justfile for now — we still need ansible-playbook to know how to reach it for the stop above. We'll remove it in step 2 once everything that touches source via ansible is done.

**Verify**:

```bash
ssh deploy@<source IP> 'docker ps --format "{{.Names}}"'
```

Expected: empty (or only `node-exporter` / `promtail`, which we leave running until step 12).

The rest of the fleet should now be on 3/4 validator quorum. Check Grafana: blocks still being produced, just slower (proposer times out every ~4 blocks until handover completes).

## Step 2. Update inventory, host_vars, and Justfile

Swap source out, target in. From here on, source is reached only by direct SSH; ansible no longer knows about it.

1. Edit `inventory`: in every group source belongs to, replace `<source IP>` with `<target IP>`. **Position matters in `[perp-liquidator-mainnet]`** — instance index is derived from list position, so insert target where source was.
2. Edit `host_vars/<target IP>.yml` to add the mainnet flags (mirror `host_vars/<source IP>.yml`):

   ```yaml
   dango_networks:
     - mainnet
   cloudflare_lb_enabled: true
   ```

3. Delete `host_vars/<source IP>.yml`.
4. Edit `Justfile`: in the four mainnet recipes (`deploy-mainnet`, `stop-mainnet`, `restart-mainnet`, `remove-deploy-lock-mainnet`), replace `<source IP>` with `<target IP>` in `--limit`.
5. Edit `deploy-hyperlane-mainnet`: replace the `validator <source IP> 1` line with `validator <target IP> 1`.

**Verify**:

```bash
grep -c "<source IP>" inventory Justfile host_vars/  # should print 0 each
grep "<target IP>" inventory                          # should print one line per group
```

## Step 3. Deploy infrastructure services on target

Postgres, ClickHouse, Traefik, Cloudflared, and Dozzle each have their own playbook. Run them all with `--limit <target IP>` so the existing fleet hosts are left alone.

```bash
uv run ansible-playbook db.yml          --limit <target IP>
uv run ansible-playbook clickhouse.yml  --limit <target IP>
uv run ansible-playbook traefik.yml     --limit <target IP>
uv run ansible-playbook cloudflared.yml --limit <target IP>
uv run ansible-playbook dozzle.yml      --limit <target IP>
```

**Verify**:

```bash
ssh deploy@<target IP> 'docker ps --format "{{.Names}}" | sort'
```

Expected: includes `postgres`, `postgres-exporter`, `clickhouse`, `traefik`, `dozzle`, and one `cloudflared-…` container. (The cloudflared container name varies by tunnel ID.)

## Step 4. Set up temporary SSH from source to target

The deploy user's private key only exists encrypted in the vault, so we generate a one-shot ed25519 keypair on source for the rsync transfer in step 6. The comment `node-migrate-temp` makes it easy to remove from target's `authorized_keys` in step 12.

```bash
ssh deploy@<source IP> 'ssh-keygen -t ed25519 -N "" -f ~/.ssh/migrate_key -C node-migrate-temp && cat ~/.ssh/migrate_key.pub'
# copy the printed pubkey, then append it on target:
ssh deploy@<target IP> 'cat >> ~/.ssh/authorized_keys' <<< '<paste pubkey>'
```

**Verify**:

```bash
ssh deploy@<source IP> 'ssh -i ~/.ssh/migrate_key -o StrictHostKeyChecking=accept-new deploy@<target IP> hostname'
```

Expected: prints target's hostname.

## Step 5. Stop postgres and clickhouse on target

Step 3 left them running; rsync needs a quiescent data directory.

```bash
ssh deploy@<target IP> 'cd ~/psql && docker compose stop'
ssh deploy@<target IP> 'cd ~/clickhouse && docker compose stop'
```

**Verify**:

```bash
ssh deploy@<target IP> 'docker ps --format "{{.Names}}" | grep -E "postgres|clickhouse"'
```

Expected: empty.

## Step 6. Rsync chain and database data from source to target

Four directories to copy. The cometbft rsync excludes the three validator-identity files — those move separately in step 9 to ensure the validator key exists on exactly one node at every point in time.

Run from your laptop; each command opens an SSH session on source which then rsyncs to target via the temporary key from step 4.

```bash
# 6a. cometbft (block store, addrbook, wal) — exclude validator identity files
ssh deploy@<source IP> 'rsync -aHv --delete \
  -e "ssh -i ~/.ssh/migrate_key -o StrictHostKeyChecking=accept-new" \
  --exclude=cometbft/config/priv_validator_key.json \
  --exclude=cometbft/config/node_key.json \
  --exclude=cometbft/data/priv_validator_state.json \
  ~/mainnet/ deploy@<target IP>:~/mainnet/'

# 6b. ~/deployments/ — orchestration metadata + .env. Target's next deploy (step 8) reads
# POSTGRES_DATABASE, CLICKHOUSE_DATABASE, DANGO_DIRECTORY, COMETBFT_DIRECTORY out of the rsynced .env
# via read_current_deploy.yml. HOSTNAME/WIREGUARD_IP/TAILSCALE_IP get re-templated by the deploy
# (no need to edit them here).
ssh deploy@<source IP> 'rsync -aHv --delete \
  -e "ssh -i ~/.ssh/migrate_key" \
  ~/deployments/ deploy@<target IP>:~/deployments/'

# 6c. postgres (indexer state)
ssh deploy@<source IP> 'rsync -aHv --delete \
  -e "ssh -i ~/.ssh/migrate_key" \
  ~/psql/data/ deploy@<target IP>:~/psql/data/'

# 6d. clickhouse (analytics)
ssh deploy@<source IP> 'rsync -aHv --delete \
  -e "ssh -i ~/.ssh/migrate_key" \
  ~/clickhouse/data/ deploy@<target IP>:~/clickhouse/data/'
```

**Verify**:

```bash
SRC_DEPLOY=$(ssh deploy@<source IP> 'jq -r .current_deployment ~/deployments/mainnet.json')
ssh deploy@<target IP> "ls ~/mainnet/$SRC_DEPLOY/cometbft/config/"
```

Expected: lists `addrbook.json`, `app.toml`, `config.toml`, `genesis.json` — but **not** `priv_validator_key.json` or `node_key.json` (those were excluded).

```bash
ssh deploy@<target IP> "ls ~/mainnet/$SRC_DEPLOY/cometbft/data/" | grep -v priv_validator_state.json
ssh deploy@<target IP> "du -sh ~/mainnet/$SRC_DEPLOY/cometbft/data ~/mainnet/$SRC_DEPLOY/dango/data ~/psql/data ~/clickhouse/data"
```

Expected: data sizes roughly match source.

## Step 7. Restart postgres and clickhouse on target

```bash
ssh deploy@<target IP> 'cd ~/psql && docker compose start'
ssh deploy@<target IP> 'cd ~/clickhouse && docker compose start'
```

**Verify**:

```bash
ssh deploy@<target IP> 'docker ps --format "{{.Names}}\t{{.Status}}" | grep -E "postgres|clickhouse"'
```

Expected: `postgres`, `postgres-exporter`, `clickhouse` all show `(healthy)` after a few seconds.

```bash
ssh deploy@<target IP> "docker exec postgres psql -U postgres -lqt | grep dango_$SRC_DEPLOY"
```

Expected: lists the database `dango_<source-deployment>` (rsynced from source). Target's next deploy will read this name from the rsynced `.env` and connect dango to it.

## Step 8. Deploy mainnet (target + 3 other validators)

This is target's first dango+cometbft deploy. It also re-templates configs on the other three validators — they pick up target's node ID into their `persistent_peers` and drop source.

```bash
just deploy-mainnet
```

> [!WARNING]
> **Brief consensus pause during deploy.**
>
> The play runs in parallel; cometbft restarts on all four nodes within a few seconds of each other. Quorum is briefly lost. A single missed block is normal; sustained halt means a container is stuck somewhere — check Grafana and `docker logs`.

What happens on target during this play:

- `read_current_deploy.yml` reads `~/deployments/mainnet.json` (rsynced from source) and pulls `DANGO_DIRECTORY`, `COMETBFT_DIRECTORY`, `POSTGRES_DATABASE`, `CLICKHOUSE_DATABASE` from source's `.env` — so target's dango talks to the rsynced data and rsynced databases.
- `cometbft_keys.yml` finds no `priv_validator_key.json` (we excluded it in step 6) → generates a fresh **non-validator** keypair (own node_key.json + a new priv_validator_key.json that is _not_ in the validator set). Target joins as a regular full node.
- The play's `check_cometbft_sync.yml` waits for target to sync to head and to have 3 peers (the other validators); it'll fail the deploy if either condition isn't met within ~5 minutes.

**Verify**:

Mainnet runs with `expose_ports: false`, so cometbft RPC is **not** bound to the host's localhost — query it inside the container, with `jq` on your laptop:

```bash
ssh deploy@<target IP> \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/status' \
  | jq '.result.sync_info | {catching_up, latest_block_height}'
```

Expected: `catching_up: false`, height matches the rest of the fleet.

```bash
ssh deploy@<target IP> \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/net_info' \
  | jq .result.n_peers
```

Expected: `3` (the other three mainnet validators).

## Step 9. Validator key + state handover

This is the slashable step — the validator key must exist on **exactly one** running cometbft. Sequence:

1. Move `priv_validator_key.json` AND `priv_validator_state.json` from source to target. Both are required: the state file records the last height/round/step the key signed; without it, target would reset to height 0 and could double-sign blocks source already signed.
2. Delete the key file on source so even if source is started later, it can't sign.

> [!WARNING]
> **Do not skip `priv_validator_state.json`.** Skipping it is the textbook double-sign mistake.

```bash
SRC_DEPLOY=$(ssh deploy@<source IP> 'jq -r .current_deployment ~/deployments/mainnet.json')
TGT_DEPLOY=$(ssh deploy@<target IP> 'jq -r .current_deployment ~/deployments/mainnet.json')

# 9a. priv_validator_key.json: source → target (pipe via your laptop)
ssh deploy@<source IP> "cat ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json" \
  | ssh deploy@<target IP> "cat > ~/mainnet/$TGT_DEPLOY/cometbft/config/priv_validator_key.json && chmod 600 ~/mainnet/$TGT_DEPLOY/cometbft/config/priv_validator_key.json"

# 9b. priv_validator_state.json: source → target
ssh deploy@<source IP> "cat ~/mainnet/$SRC_DEPLOY/cometbft/data/priv_validator_state.json" \
  | ssh deploy@<target IP> "cat > ~/mainnet/$TGT_DEPLOY/cometbft/data/priv_validator_state.json && chmod 600 ~/mainnet/$TGT_DEPLOY/cometbft/data/priv_validator_state.json"

# 9c. Delete the key on source (so source can never sign again, even if started)
ssh deploy@<source IP> "rm ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json"
```

> [!NOTE]
> `$TGT_DEPLOY` will equal `$SRC_DEPLOY` — target inherited source's deployment timestamp via the rsynced `mainnet.json` in step 6. The `du` paths are the same on both sides.

**Verify**:

```bash
# the file is gone on source:
ssh deploy@<source IP> "ls ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json" 2>&1
# expected: "No such file or directory"

# the validator address on target now matches what source's was:
ssh deploy@<target IP> "docker run --rm -v ~/mainnet/$TGT_DEPLOY/cometbft:/root/.cometbft \
  ghcr.io/left-curve/left-curve/cometbft:v0.38.21 \
  cometbft show-validator --home /root/.cometbft"
# expected: prints the public key your validator slot expected (cross-check Grafana / chain validator list)
```

## Step 10. Restart target's cometbft and verify signing

Cometbft has the new key on disk but needs to be restarted to load it.

```bash
uv run ansible-playbook restart-services.yml -e dango_network=mainnet --limit <target IP>
```

**Verify** that target is signing recent blocks. Query a peer (e.g. hetzner1 at `100.126.8.2`) — same docker-exec pattern as step 8:

```bash
ssh deploy@100.126.8.2 \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/block' \
  | jq '.result.block.last_commit.signatures[] | select(.block_id_flag == 2) | .validator_address'
```

Expected: target's validator address appears in the signatures list within ~10–30 seconds. Repeat the query a few times to confirm it consistently signs.

Also check Grafana's "validator missed blocks" panel — your slot should drop from 100% missed (during steps 1–9) back to ~0%.

## Step 11. Migrate hyperlane validator role to target

Source ran `mainnet-validator-1` (per its position in `[hyperlane]`). Stop it on source via direct `docker compose` — `just stop-hyperlane` won't work here because source is no longer in inventory, so ansible's `--limit` would match nothing. Then deploy on target via the parameterized just recipe (which targets by IP).

```bash
ssh deploy@<source IP> 'cd ~/hyperlane-agents/mainnet-validator-1 && docker compose down'
just deploy-hyperlane mainnet validator <target IP> 1
```

The KMS key and dango signer secrets are vaulted by validator index, not by host, so target reuses the same identity — no secret rotation needed.

**Verify**:

```bash
ssh deploy@<target IP> 'docker logs mainnet-validator-1 --tail 50 2>&1 | grep -i "checkpoint\|posted\|started"'
```

Expected: a "successfully posted" or "starting validator" line within ~1 minute of starting.

## Step 12. Wipe source

The host is no longer used by the fleet. Either repurpose it (re-run [`NEW_SERVER_SETUP.md`](NEW_SERVER_SETUP.md) from step 5 onwards after wiping) or hand it back to the vendor.

```bash
# Stop residual systemd-managed compose stacks
ssh debian@<source IP> 'sudo systemctl disable --now \
  postgres-compose.service \
  clickhouse-compose.service \
  traefik-compose.service \
  cloudflared-compose.service' || true

# Wipe application and identity state
ssh debian@<source IP> 'sudo rm -rf /home/deploy/{mainnet,deployments,psql,clickhouse,traefik,hyperlane-agents,.ssh/migrate_key,.ssh/migrate_key.pub}'

# Remove the temporary migrate-key grant from target's authorized_keys (the corresponding
# private key on source is gone, but clean up the trust for hygiene).
ssh deploy@<target IP> 'sed -i.bak "/node-migrate-temp$/d" ~/.ssh/authorized_keys && rm ~/.ssh/authorized_keys.bak'
```

Remove the host from tailscale (admin console: <https://login.tailscale.com/admin/machines> → select source → Remove). Re-running `wireguard.yml` after step 2's inventory edit already pulled source out of every other host's `wg0.conf`.

## Done

Target is the new mainnet validator in source's slot, taking over hyperlane validator-1 and source's perp-liquidator instance. Watch Grafana for ~24h to confirm: steady block-proposal rate, indexer lag near zero, no log spikes.
