# WireGuard Role

This Ansible role sets up a WireGuard mesh network across all hosts in the inventory.

## Features

- Installs WireGuard and wireguard-tools packages
- Automatically fetches public IPs from each server (not committed to git)
- Configures UFW firewall rules to allow WireGuard traffic between nodes
- Generates WireGuard private/public key pairs if they don't exist
- Creates `/etc/wireguard/wg0.conf` with mesh network configuration
- Uses static WireGuard IPs defined in `host_vars`
- Enables and starts the WireGuard service
- Verifies connectivity by pinging all other nodes

## Requirements

- Debian-based systems
- UFW firewall installed
- Root access (playbook must have `become: true`)
- **Each host must have `wireguard_ip` defined in its `host_vars` file**

## Variables

**Required (must be set in host_vars):**
- `wireguard_ip`: The WireGuard IP address for this host (e.g., "10.99.0.1")

**Optional (see `defaults/main.yml`):**
- `wireguard_port`: UDP port for WireGuard (default: 51820)
- `wireguard_persistent_keepalive`: Keepalive interval in seconds (default: 25)

## IP Assignment

WireGuard IPs must be pre-configured in each host's `host_vars` file.

## Usage

Run the playbook:

```bash
ansible-playbook -i inventory wireguard.yml
```

## Security Notes

- Private keys are stored in `/etc/wireguard/privatekey` with mode 0600
- Public IPs are fetched at runtime and not committed to git
- UFW rules only allow WireGuard traffic from known peer IPs
- Configuration file `/etc/wireguard/wg0.conf` has mode 0600

## Using WireGuard IPs in Other Roles

The `wireguard_ip` variable is available in each host's vars and can be referenced in other roles to communicate over the secure WireGuard network.

Example:
```yaml
- name: Connect to service on WireGuard network
  uri:
    url: "http://{{ hostvars[item].wireguard_ip }}:8080/api"
  loop: "{{ groups['all'] }}"
```

## Files Created

- `/etc/wireguard/privatekey` - WireGuard private key
- `/etc/wireguard/publickey` - WireGuard public key
- `/etc/wireguard/wg0.conf` - WireGuard interface configuration
