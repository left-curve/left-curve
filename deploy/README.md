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
