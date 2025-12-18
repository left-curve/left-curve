# deploy

## Add a new user

- Add the username in `group_vars/all/main.yml` in the `ssh_users` section

- Add the public key in `roles/users/files/authorized_keys/<username>.pub`

- Run `ansible-playbook users.yml`

## Install a new server

- Add the debian-deploy key:

```bash
ssh-copy-id -i ~/.ssh/debian_deploy.pub username@public_ip
```

- Add the host in `inventory` using its public IP

  ```bash
  ansible-playbook init-debian-user.yml --limit <public IP> -e ansible_user=<your remote user account>
  ansible-playbook common.yml --limit <public IP>
  ansible-playbook tailscale.yml --limit <public IP>
  ```

- Ensure tailscale IP is up and you can see the server

- Replace the public IP with the private IP in `inventory`, create a file
`host_vars/<tailscale IP>.yml` and `hostname`, `wireguard_ip`, `tailscale_ip`.

- Enable Wireguard on all hosts

  ```bash
  ansible-playbook wireguard.yml
  ```

- Install default packages, users, and things like docker using:

  ```bash
  ansible-playbook playbook.yml --limit <tailscale IP>
  ```

## Setup Ansible Vault

### First time setup (macOS)

Add vault password to Keychain:

```bash
security add-generic-password \
  -a ansible \
  -s ansible-vault/default \
  -w 'ASK_TEAM_FOR_PASSWORD'
```

This shows you have the right password:

```bash
❯ ./vault-password.sh|sha256
2f919beb6554c5149ebfdbf03076bed7796fb6853e1d9993bfa259622c7a84e0
```

Make also sure you have ssh-agent and added your key with ssh-add before
running ansible-playbook, else you'll get `Permission denied (publickey)`.

You must rerun `ssh-add` after you rebooted.

### Root access

No one should need debian/sudo access to the servers, this is a critical
access. But here is the process.

Add debian password to Keychain:

```bash
security add-generic-password \
  -a ansible \
  -s ansible-debian/default \
  -w 'ASK_TEAM_FOR_PASSWORD'
```

This shows you have the right password:

```bash
❯ ./debian-password.sh|sha256
b82a3865821fb1c7072cf58ca641811fd814c892109963f54fce675e7e9cfca5
```

Make also sure you have ssh-agent and added your key with ssh-add before
running ansible-playbook, else you'll get `Permission denied (publickey)`.

You must rerun `ssh-add` after you rebooted.

## Using the deploy key (vaulted)

The private key is encrypted in `group_vars/all/deploy_key.vault`, load it
directly into ssh-agent without writing to disk:

`just add-deploy-key`

Notes:
- Ensure `ssh-agent` is running in your shell (`eval $(ssh-agent -s)` if needed).

## Manual Cosign Verification

Run this after deployments if you need to validate an image digest manually:

```bash
cosign verify \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp "https://github.com/left-curve/left-curve/.github/workflows/rust.yml@refs/heads/main" \
  ghcr.io/left-curve/left-curve/dango@sha256:<digest>
```

## Cloudflare tunnels and load balancers

Those are deployed differently for testnet/devnet and PR review apps.

### PR review apps

When `cloudflare_tunnel_enabled` is set to true, the review app docker compose
includes a cloudflare tunnel container. Then we create CNAME for each service,
to that specific "PR-container" tunnel.

The cloudflared container has a config, routing to containers based on host.

[user] -> (( cloudflare )) -> [cloudflared PR container] -> [destination PR container]

### devnet/testnet

Each host has a specific cloudflare tunnel name with the hostname. A
`cloudflare` docker network is created. The host running traefik includes the
cloudflare network.

We add a new `traefik` config file, so :80 and :443 and connected to the PR
containers. It routes those port to proper container services based on
hostname.

The cloudflared container has a config, routing to containers based on host.

[user] -> (( cloudflare )) -> [cloudflared system container] -> [system traefik] -> [destination container]
