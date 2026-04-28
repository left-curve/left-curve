---
paths:
  - "deploy/**"
---
# Deploy Rules (Ansible/SSH)

## Critical Rules

### Never Reboot Servers

Do not reboot servers without explicit user permission. Ask the user to reboot
manually or confirm before executing `reboot` commands.

### SSH Connection Pattern

Connect as the user's username, use `sudo -u deploy` for deploy commands:

```bash
ssh hetzner1 "sudo -u deploy docker compose logs"
```

Do NOT try to SSH directly as `deploy` user — it won't work.

### Confirm Destructive SSH Commands

Any command executed over SSH that edits files, modifies state, or performs
destructive actions (e.g., `docker rm`, `systemctl restart`, `rm`, database
changes) must be confirmed with the user before execution. Read-only commands
(e.g., `docker ps`, `systemctl status`, `cat`, `ls`) do not require confirmation.

## State Changes via Playbooks Only

All state-changing operations (starting, stopping, restarting, or deploying
services) must use existing Ansible playbooks, not direct `docker compose` or
`systemctl` commands. Playbooks may use multiple compose files, host-specific
overrides, or additional provisioning steps that a bare `docker compose up`
would miss.

Key playbooks:

- `traefik.yml` — Traefik reverse proxy (uses host-specific compose overrides)
- `db.yml` — PostgreSQL
- `clickhouse.yml` — ClickHouse
- `promtail.yml` — Promtail + Node-exporter (both in one playbook)
- `full-app.yml` — Full application deployment
- `restart-services.yml` — Restart dango/cometbft services
- `stop-services.yml` — Stop services

Most playbooks require both the `deploy` and `debian` SSH keys. Use `--limit <ip>`
to target a specific server.

## Linting

Always lint YAML files after modifications and before commits:

```bash
just lint                    # Lint all playbooks and roles
just lint-file path/to/file  # Lint specific files
```

Uses `yamllint` via uvx (no installation needed). Fix any linting errors before
committing changes.

## Ansible Patterns

### Home Directory Resolution

**Problem**: Using `ansible_facts['env']['HOME']` in role defaults or templates
can resolve to the wrong home directory when:

- The playbook uses `remote_user: debian` with `become: true` and `become_user: "{{ deploy_user }}"`
- Facts are gathered as the `remote_user`, so `HOME` is that user's home (e.g., `/home/debian` or `/root`)
- But files are deployed to the `become_user`'s home (e.g., `/home/deploy`)

This causes systemd services to fail with `status=200/CHDIR` because
`WorkingDirectory` points to a non-existent or inaccessible path.

**Solution**: Dynamically look up the deploy user's home directory using `getent`:

```yaml
pre_tasks:
  - name: Get deploy user info
    getent:
      database: passwd
      key: "{{ deploy_user }}"

  - name: Set deploy_home fact
    set_fact:
      deploy_home: "{{ ansible_facts.getent_passwd[deploy_user][4] }}"
```

Then use `{{ deploy_home }}` in role defaults instead of `{{ ansible_facts['env']['HOME'] }}`:

```yaml
# Good
promtail_dir: "{{ deploy_home }}/promtail"

# Bad - will resolve incorrectly with become
promtail_dir: "{{ ansible_facts['env']['HOME'] }}/promtail"
```

Use `ansible_facts.getent_passwd` instead of `getent_passwd` to avoid deprecation
warnings about `INJECT_FACTS_AS_VARS`.

### Tailscale Startup Timing

**Problem**: Systemd services with `After=tailscaled.service` may start before
Tailscale is actually connected. The `tailscaled.service` reports as "started"
immediately when the daemon launches, but the network interface may not be ready
for several seconds.

This causes Docker containers to fail binding to Tailscale IPs because the
interface doesn't exist yet.

**Solution**: Add an `ExecStartPre` check that waits for Tailscale to be connected:

```ini
ExecStartPre=/bin/sh -c 'until tailscale status --peers=false 2>/dev/null | grep -q "^100\\."; do sleep 1; done'
ExecStart=/usr/bin/docker compose up -d --remove-orphans
```

This loops until `tailscale status` shows an IP starting with `100.` (Tailscale
CGNAT range), indicating the connection is ready.

## Debugging Systemd Services

When systemd services fail to start after reboot:

1. Check service status: `systemctl status <service-name>`
2. Look for `status=200/CHDIR` — indicates WorkingDirectory issue
3. Verify the path in the service file: `cat /etc/systemd/system/<service>.service | grep WorkingDirectory`
4. Compare with actual file locations: `find /home -name 'docker-compose.yml' -path '*<service>*'`
