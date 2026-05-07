# Node Migration

Step-by-step runbook for moving an existing mainnet validator role from one server to another. The **source** server is currently running the chain (validating, hosting traefik/cloudflared/postgres/clickhouse, running its hyperlane validator instance, etc.). The **target** server has finished [`NEW_SERVER_SETUP.md`](NEW_SERVER_SETUP.md) — i.e. it's on the tailnet, in the WireGuard mesh, has the deploy user and Docker, and is running node-exporter + promtail, but has no application services yet.

The objective is to move all chain data, indexer state, and the validator identity from source to target with minimal disruption to mainnet. The other 3 mainnet validators are not touched during the migration — they keep running with their current config. Target inherits source's cometbft p2p identity (via the rsynced `node_key.json`), so once target comes online the rest of the fleet talks to it without any peer-list reconfiguration.

## Prerequisites

- **Source's tailscale IP** and **target's tailscale IP**.
- **Source and target hostnames** (e.g. `inter1`, `hetzner5`).
- **Vault access** and keys in agent (`just add-deploy-key && just add-debian-key`).
- **Disk space on target**: at least 1.5× the sum of source's `~/mainnet/`, `~/psql/data/`, and `~/clickhouse/data/`. Run `ssh deploy@$source_ip 'du -sh ~/mainnet ~/psql/data ~/clickhouse/data'` to check.
- **Time window**: budget ~60–90 minutes. Source is offline from step 1 onward; the chain runs on 3/4 quorum during that window, with source's proposer slot timing out every ~4 blocks.

All commands assume `deploy/` is your working directory. Ansible runs through `uv`.

Commands throughout this runbook use shell variables `$source_ip` and `$target_ip`. Set them once at the top of your shell session — replace the example values with the actual IPs:

```bash
source_ip=100.89.7.33     # source server's tailscale IP
target_ip=100.72.62.100   # target server's tailscale IP
```

## Step 1. Stop all services on source

Source is no longer needed for consensus from this point — it just hosts data we're about to copy off. We do this before the inventory edit (step 2) so we can use `stop-services.yml` for the dango+cometbft stack: that play also pauses the Kuma uptime monitor, which silences alerts during the migration window. Postgres + ClickHouse have no equivalent stop playbook, so they get stopped via direct `docker compose`.

```bash
uv run ansible-playbook stop-services.yml \
  -e dango_network=mainnet \
  --limit $source_ip

ssh deploy@$source_ip 'cd ~/psql       && docker compose stop'
ssh deploy@$source_ip 'cd ~/clickhouse && docker compose stop'
```

**Verify**:

```bash
ssh deploy@$source_ip 'docker ps --format "{{.Names}}"'
```

Expected: only `node-exporter` / `promtail` remain.

The rest of the fleet is now on 3/4 validator quorum. Check Grafana: blocks still being produced, just slower because the proposer slot of `<source>` will time out every ~4 blocks.

## Step 2. Update inventory, host_vars, and Justfile

Swap source out, target in. From here on, source is reached only by direct SSH; ansible no longer knows about it. The shell variables `$source_ip` and `$target_ip` from your session refer to the actual IPs you'll be substituting in these files.

1. Edit `inventory`: in every group source belongs to, replace source's IP with target's IP. **Position matters in `[perp-liquidator-mainnet]`** — instance index is derived from list position, so insert target where source was.
2. Create `host_vars/<target IP>.yml` (named after target's actual IP) with the mainnet flags (mirror `host_vars/<source IP>.yml`):

   ```yaml
   dango_networks:
     - mainnet
   cloudflare_lb_enabled: true
   ```

3. Delete `host_vars/<source IP>.yml`.
4. Edit `Justfile`: in the four mainnet recipes (`deploy-mainnet`, `stop-mainnet`, `restart-mainnet`, `remove-deploy-lock-mainnet`), replace source's IP with target's IP in each `--limit` list.
5. Edit `deploy-hyperlane-mainnet`: replace `validator <source IP> 1` with `validator <target IP> 1`.

**Verify**:

```bash
grep -c "$source_ip" inventory Justfile host_vars/  # should print 0 each
grep    "$target_ip" inventory                      # should print one line per group
```

## Step 3. Set up temporary SSH from source to target

The deploy user's private key only exists encrypted in the vault, so we generate a one-shot ed25519 keypair on source for the rsync transfer in step 4. The comment `node-migrate-temp` makes it easy to remove from target's `authorized_keys` in step 10.

```bash
ssh deploy@$source_ip 'ssh-keygen -t ed25519 -N "" -f ~/.ssh/migrate_key -C node-migrate-temp && cat ~/.ssh/migrate_key.pub'
# copy the printed pubkey, then append it on target:
ssh deploy@$target_ip 'cat >> ~/.ssh/authorized_keys' <<< '<paste pubkey>'
```

**Verify**:

```bash
ssh deploy@$source_ip "ssh -i ~/.ssh/migrate_key -o StrictHostKeyChecking=accept-new deploy@$target_ip hostname"
```

Expected: prints target's hostname.

## Step 4. Initialize target's cometbft dir, then rsync chain and database data

Four directories to copy, plus one preparatory `cometbft init` on target. The init creates a non-validator `priv_validator_key.json` + a fresh `priv_validator_state.json` (height 0) — both of which the subsequent rsync deliberately leaves in place via excludes.

`node_key.json` (the p2p identity) **is** overwritten by the rsync: target inherits source's network identity so the other validators' `addrbook` keeps trusting the same `cometbft_node_id`.

Capture source's deployment timestamp once — used in this step and again in step 7.

```bash
SRC_DEPLOY=$(ssh deploy@$source_ip 'jq -r .current_deployment ~/deployments/mainnet.json')
echo "$SRC_DEPLOY"
```

### 4a. Pre-initialize target's cometbft directory

```bash
ssh deploy@$target_ip "mkdir -p ~/mainnet/$SRC_DEPLOY/cometbft && \
  docker run --rm \
  -v ~/mainnet/$SRC_DEPLOY/cometbft:/root/.cometbft \
  ghcr.io/left-curve/left-curve/cometbft:v0.38.21 \
  cometbft init --home /root/.cometbft"
```

After this, target has:

- `config/node_key.json` — target's, will be overwritten by rsync below.
- `config/priv_validator_key.json` — target's, **non-validator** key. Stays through rsync; gets overwritten by source's actual validator key in step 7.
- `config/genesis.json`, `config/config.toml` — target's defaults; rsync overwrites both.
- `data/priv_validator_state.json` — `{"height":"0",...}`. Stays through rsync; gets overwritten in step 7.

### 4b. Rsync data from source to target

Run from your laptop; each command opens an SSH session on source which then rsyncs to target via the temporary key from step 3. The outer double quotes let `$target_ip` expand locally before SSH runs the rsync; `~` and any other tilde paths still expand on the remote side because bash doesn't expand `~` inside double quotes.

```bash
# cometbft (block store, addrbook, wal, source's node_key) — `--exclude` protects target's
# freshly-init'd priv_validator_key.json and priv_validator_state.json from BOTH being copied
# and being deleted (`--delete` honors the exclude list — see rsync(1) on `--delete-excluded`).
ssh deploy@$source_ip "rsync -aHv --delete \
  -e 'ssh -i ~/.ssh/migrate_key -o StrictHostKeyChecking=accept-new' \
  --exclude=cometbft/config/priv_validator_key.json \
  --exclude=cometbft/data/priv_validator_state.json \
  ~/mainnet/ deploy@$target_ip:~/mainnet/"

# ~/deployments/ — orchestration metadata + .env. Target's full-app deploy (step 6) reads
# POSTGRES_DATABASE, CLICKHOUSE_DATABASE, DANGO_DIRECTORY, COMETBFT_DIRECTORY out of the rsynced .env
# via read_current_deploy.yml. HOSTNAME/WIREGUARD_IP/TAILSCALE_IP get re-templated by the deploy.
ssh deploy@$source_ip "rsync -aHv --delete \
  -e 'ssh -i ~/.ssh/migrate_key' \
  ~/deployments/ deploy@$target_ip:~/deployments/"

# postgres (indexer state) — target's ~/psql/data/ doesn't exist yet (db.yml runs in step 5);
# rsync creates it. The data dir's PG_VERSION marker tells postgres' entrypoint to skip initdb
# and just start on this data when db.yml brings the container up.
ssh deploy@$source_ip "rsync -aHv --delete \
  -e 'ssh -i ~/.ssh/migrate_key' \
  ~/psql/data/ deploy@$target_ip:~/psql/data/"

# clickhouse (analytics) — same story; clickhouse.yml in step 5 finds existing data and uses it.
ssh deploy@$source_ip "rsync -aHv --delete \
  -e 'ssh -i ~/.ssh/migrate_key' \
  ~/clickhouse/data/ deploy@$target_ip:~/clickhouse/data/"
```

**Verify** that source's data is on target, and that target's freshly-init'd validator-identity files survived:

```bash
ssh deploy@$target_ip "ls ~/mainnet/$SRC_DEPLOY/cometbft/config/"
ssh deploy@$target_ip "ls ~/mainnet/$SRC_DEPLOY/cometbft/data/"
```

Expected: `config/` includes `node_key.json`, `priv_validator_key.json`, `genesis.json`, `app.toml`, `config.toml`, `addrbook.json`. `data/` includes `priv_validator_state.json` plus the cometbft block-store files (`blockstore.db`, `state.db`, `cs.wal/`, etc.).

Confirm the target's `node_key.json` matches source's:

```bash
ssh deploy@$source_ip "cd ~/deployments && \
  DEPLOY=\$(jq -r .current_deployment mainnet.json) && \
  docker compose -p \$DEPLOY exec cometbft cometbft show-node-id"
ssh deploy@$target_ip "cd ~/deployments/$SRC_DEPLOY && \
  docker compose -p $SRC_DEPLOY run --rm --no-deps cometbft cometbft show-node-id"
```

Expected: same id printed by both.

## Step 5. Deploy infrastructure services on target

Postgres, ClickHouse, Traefik, Cloudflared, and Dozzle each have their own playbook. Run them all with `--limit $target_ip` so the existing fleet hosts are left alone.

Postgres and ClickHouse start against the data dirs rsynced in step 4 — their entrypoints detect existing PG_VERSION / clickhouse data and skip initialization, so source's databases come up intact. The `db` and `clickhouse` roles only manage `~/psql/{config,docker-compose.yml,.env}` and `~/clickhouse/{config,docker-compose.yml,.env}`; they never touch `data/`. Traefik/Cloudflared/Dozzle have no rsynced state and start fresh.

```bash
uv run ansible-playbook db.yml          --limit $target_ip
uv run ansible-playbook clickhouse.yml  --limit $target_ip
uv run ansible-playbook traefik.yml     --limit $target_ip
uv run ansible-playbook cloudflared.yml --limit $target_ip
uv run ansible-playbook dozzle.yml      --limit $target_ip
```

**Verify**:

```bash
ssh deploy@$target_ip 'docker ps --format "{{.Names}}" | sort'
```

Expected: includes `postgres`, `postgres-exporter`, `clickhouse`, `traefik`, `dozzle`, and one `cloudflared-…` container. (The cloudflared container name varies by tunnel ID.)

```bash
ssh deploy@$target_ip "docker exec postgres psql -U postgres -lqt | grep dango_$SRC_DEPLOY"
```

Expected: lists the database `dango_<source-deployment>` (rsynced from source). If this prints nothing, postgres re-initialized instead of picking up the rsynced data — check that `~/psql/data/PG_VERSION` exists on target.

## Step 6. First mainnet deploy on target

Run the full-app play scoped to target only, with `-e cometbft_peers=<the 3 healthy IPs>` so target's `persistent_peers` lists them by their actual node IDs. Target dials them; they accept inbound (cometbft's addrbook is non-strict) and PEX gossips target's identity through the cluster.

`just deploy-mainnet` won't work here because its `--limit` is hardcoded for the full validator set. Run `ansible-playbook` directly:

```bash
mkdir -p logs && uv run ansible-playbook full-app.yml \
  -e '{"traefik_enabled": true, "cometbft_generate_keys": true, "dex_bot_enabled": false, "github_deployments_enabled": false, "expose_ports": false, "delete_postgres_database_at_merge": false, "delete_clickhouse_database_at_merge": false, "deploy_includes_postgres": false, "deploy_includes_clickhouse": false, "chain_id": "dango-1", "dango_network": "mainnet", "system_wide_directories": true, "deploy_env": "production", "dango_image_tag": "latest", "frontend_image_tag": "latest"}' \
  -e cometbft_peers=100.126.8.2,100.66.234.16,100.76.197.30 \
  --limit $target_ip \
  2>&1 | tee logs/$(date -u +%Y%m%d%H%M%S)-deploy-target.log
```

**Verify**:

```bash
ssh deploy@$target_ip \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/status' \
  | jq '.result.sync_info | {catching_up, latest_block_height}'
```

Expected: `catching_up: false`, height matches the rest of the fleet.

```bash
ssh deploy@$target_ip \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/net_info' \
  | jq .result.n_peers
```

Expected: `3` (the three healthy mainnet validators).

## Step 7. Validator key + state handover

This is the slashable step — the validator key must exist on **exactly one** running cometbft. Sequence:

1. Move `priv_validator_key.json` AND `priv_validator_state.json` from source to target. Both are required: the state file records the last height/round/step the key signed; without it, target would reset to height 0 and could double-sign blocks source already signed.
2. Delete the key file on source so even if source is started later, it can't sign.

> [!WARNING]
> **Do not skip `priv_validator_state.json`.** Skipping it is the textbook double-sign mistake.

```bash
SRC_DEPLOY=$(ssh deploy@$source_ip 'jq -r .current_deployment ~/deployments/mainnet.json')

# 7a. priv_validator_key.json: source → target (rsync directly via the migrate_key from
# step 3 so the validator private key never transits through your laptop). `-a` preserves
# source's 0600 file mode.
ssh deploy@$source_ip "rsync -av \
  -e 'ssh -i ~/.ssh/migrate_key' \
  ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json \
  deploy@$target_ip:~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json"

# 7b. priv_validator_state.json: source → target
ssh deploy@$source_ip "rsync -av \
  -e 'ssh -i ~/.ssh/migrate_key' \
  ~/mainnet/$SRC_DEPLOY/cometbft/data/priv_validator_state.json \
  deploy@$target_ip:~/mainnet/$SRC_DEPLOY/cometbft/data/priv_validator_state.json"

# 7c. Delete the key on source (so source can never sign again, even if started)
ssh deploy@$source_ip "rm ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json"
```

> [!NOTE]
> Target's deployment paths are the same as source's — target inherited source's `~/deployments/mainnet.json` and `.env` via the step 4 rsync, so `$SRC_DEPLOY` is the directory name on both sides.

**Verify**:

```bash
ssh deploy@$source_ip "ls ~/mainnet/$SRC_DEPLOY/cometbft/config/priv_validator_key.json" 2>&1
# expected: "No such file or directory"

ssh deploy@$target_ip "cd ~/deployments/$SRC_DEPLOY && \
  docker compose -p $SRC_DEPLOY exec cometbft cometbft show-validator"
# expected: prints the public key your validator slot has historically used
# (cross-check against Grafana / chain validator list).
# Note: cometbft is still loaded with the non-validator key in memory at this point —
# show-validator reads from disk so it'll already show the handed-over pubkey.
```

## Step 8. Restart target's cometbft and verify signing

CometBFT now has the new key on disk but needs to be restarted to load it. `just restart-mainnet`'s `--limit` is hardcoded for the full validator set, so we run the restart play directly to scope to target only.

```bash
uv run ansible-playbook restart-services.yml \
  -e dango_network=mainnet \
  --limit $target_ip
```

**Verify** target is signing recent blocks. Query a peer (e.g. hetzner1 at `100.126.8.2`) — same docker-exec pattern as step 6:

```bash
ssh deploy@100.126.8.2 \
  'docker exec $(docker ps -q --filter label=service_name=cometbft | head -1) \
  curl -s http://localhost:26657/block' \
  | jq '.result.block.last_commit.signatures[] | select(.block_id_flag == 2) | .validator_address'
```

Expected: target's validator address appears in the signatures list within ~10–30 seconds. Repeat the query a few times to confirm it consistently signs.

Also check Grafana's "validator missed blocks" panel — your slot should drop from 100% missed (during steps 1–7) back to ~0%.

## Step 9. Migrate hyperlane validator role to target

Source ran `mainnet-validator-1` (per its position in `[hyperlane]`). Stop it on source via direct `docker compose` — `just stop-hyperlane` won't work here because source is no longer in inventory. Then deploy on target via the parameterized just recipe (which targets by IP).

```bash
ssh deploy@$source_ip 'cd ~/hyperlane-agents/mainnet-validator-1 && docker compose down'
just deploy-hyperlane mainnet validator $target_ip 1
```

The KMS key and dango signer secrets are vaulted by validator index, not by host, so target reuses the same identity — no secret rotation needed.

**Verify**:

```bash
ssh deploy@$target_ip 'docker logs mainnet-validator-1 --tail 50 2>&1 | grep -i "checkpoint\|posted\|started"'
```

Expected: a "successfully posted" or "starting validator" line within ~1 minute of starting.

## Step 10. Wipe source

The host is no longer used by the fleet. Either repurpose it (re-run [`NEW_SERVER_SETUP.md`](NEW_SERVER_SETUP.md) from step 5 onwards after wiping) or hand it back to the vendor.

```bash
# Stop residual systemd-managed compose stacks
ssh debian@$source_ip 'sudo systemctl disable --now \
  postgres-compose.service \
  clickhouse-compose.service \
  traefik-compose.service \
  cloudflared-compose.service' || true

# Wipe application and identity state
ssh debian@$source_ip 'sudo rm -rf /home/deploy/{mainnet,deployments,psql,clickhouse,traefik,hyperlane-agents,.ssh/migrate_key,.ssh/migrate_key.pub}'

# Remove the temporary migrate-key grant from target's authorized_keys (the corresponding
# private key on source is gone, but clean up the trust for hygiene).
ssh deploy@$target_ip 'sed -i.bak "/node-migrate-temp$/d" ~/.ssh/authorized_keys && rm ~/.ssh/authorized_keys.bak'
```

Remove the host from tailscale (admin console: <https://login.tailscale.com/admin/machines> → select source → Remove). Re-running `wireguard.yml` after step 2's inventory edit already pulled source out of every other host's `wg0.conf`.

## Done

Target is the new mainnet validator in source's slot, taking over hyperlane validator-1 and source's perp-liquidator instance. Watch Grafana for ~24h to confirm: steady block-proposal rate, indexer lag near zero, no log spikes.
