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
- `just setup`: Create venv, install Ansible, collections, and roles.
- `just cold-provision` | `just provision`: Run base provisioning (the latter skips `setup` tag).
- `just deploy-devnet` | `just deploy-testnet` | `just deploy-preview-latest`: Deploy full app to targets.
- `ansible-playbook <playbook>.yml --limit <host>`: Scope to a single host. Add `--check --diff` for dry-runs.
- `just deploy-monitoring` | `just deploy-clickhouse` | `just deploy-tailscale`: Deploy specific subsystems.

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
