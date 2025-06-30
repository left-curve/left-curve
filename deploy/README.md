
# Install a new server

- Add the host in `inventory` using its public IP
- Install tailscale using:

```
ansible-playbook tailscale.yml --limit <public IP>
```

- Replace the public IP with the private IP in `inventory`
- Install default packages, users and things like docker using:

```
ansible-playbook playbook.yml --limit <private IP>
```
