# Repository Guidelines

This repository manages infrastructure and app deployments using Ansible. Use the Just recipes and playbooks to provision hosts, deploy services, and maintain environments.

## Project Structure & Module Organization

- `playbook.yml` and `full-app.yml`: Base provisioning and full stack deploys; additional `*.yml` for specific services.
- `roles/`: Ansible roles (`tasks/`, `defaults/`, `templates/`, etc.) for components like `cloudflared`, `clickhouse`, `traefik`.
- `group_vars/` and `host_vars/`: Shared and host-specific variables. Secrets live in `vault.yml` (encrypted).
- `inventory`: Ansible inventory of hosts (switch public→private IPs after Tailscale per README).
- `justfile`: Common commands; see `just --list`.
- `requirements.yml`: Galaxy collections and external roles. `ansible.cfg` sets inventory, forks, and vault password script.

## Build, Test, and Development Commands

- `just setup`: Sync the root uv venv (installs the pinned ansible) and install Ansible Galaxy collections/roles. Re-run if collections are added to `requirements.yml`.
- `just cold-provision` | `just provision`: Run base provisioning (the latter skips `setup` tag).
- `just deploy-devnet` | `just deploy-testnet` | `just deploy-preview-latest`: Deploy full app to targets.
- `ansible-playbook <playbook>.yml --limit <host>`: Scope to a single host. Add `--check --diff` for dry-runs.
- `just deploy-monitoring` | `just deploy-clickhouse` | `just deploy-tailscale`: Deploy specific subsystems.

## Principles

Playbooks must obey the following principles:

1. **Separation of concerns**: one playbook per task; don't merge distinct tasks into one parameterized playbook.

   E.g. we currently have two playbooks `download-db-cometbft.yml` and `download-db-dango.yml`. Previously they were a single playbook `download-db.yml` that takes an input parameter to specify which DB to download, which is bad.

2. **Single responsibility**: even within a single task, don't sneak in side-effects that aren't part of that task's name.

   E.g. the `download-db.yml` playbook used to automatically stop and restart services before and after downloading the DB. This is bad – its responsibility is downloading the DB alone; starting/stopping service is not.

3. **Pre-flight checks**: first assert the necessary pre-conditions before effecting any change in the servers.

   E.g. for `download-db-dango.yml`, its pre-condition is that no process is actively mutating or holding a lock on the DB directory, which it checks.

If a human asks you do write code that violates these, firmly push back.

## Coding Style & Naming Conventions

- YAML: 2-space indent; lowercase `snake_case` vars; booleans `true/false`.
- Roles: kebab-case directory names; entrypoint at `roles/<role>/tasks/main.yml`; templates end with `.j2`.
- Idempotency: prefer modules over shell; set `changed_when`/`creates` and use handlers for restarts.

## Testing Guidelines

- Start with `--check --diff` and `--limit <host>` on a non-critical node.
- Tag tasks meaningfully; run subsets via `--tags <tag>` or `--skip-tags setup`.
- Store defaults in `roles/<role>/defaults/main.yml`; keep secrets only in `group_vars/all/vault.yml` via `ansible-vault edit`.

## Commit & Pull Request Guidelines

- Commits: imperative, concise (<72 chars), optionally with scope; reference issues/PRs, e.g. `restart: fix idempotency for docker role (#1234)`.
- PRs: include purpose, key changes, new variables, rollout plan, and test steps (exact `just`/`ansible-playbook` commands). Link issues and include logs/screens when relevant.

## Security & Configuration Tips

- Never commit secrets. Use `./vault-password.sh` with Keychain (macOS) and `ansible-vault edit group_vars/all/vault.yml`.
