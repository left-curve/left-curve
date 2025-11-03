# deploy

## Install a new server

- Add the host in `inventory` using its public IP

  ```bash
  ansible-playbook playbook.yml --limit <public IP>
  ansible-playbook tailscale.yml --limit <public IP>
  ```

- Ensure tailscale IP is up and you can see the server

- Replace the public IP with the private IP in `inventory`

- Install default packages, users, tailscale and things like docker using:

  ```bash
  ansible-playbook playbook.yml --limit <private IP>
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

`cloudflare_tunnel_enabled` is set to true, the review app docker compose
includes a specific cloudflare tunnel. Then we create CNAME for each service,
per PR, to that specific tunnel.

### devnet/testnet

Each host has a specific cloudflare tunnel name with the hostname. A
`cloudflare` docker network is created. The host running traefik includes the
cloudflare network.
