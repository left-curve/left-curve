# SOPS deploy secrets scaffold

This is the repo-local SOPS plan for deploy secrets. It is a scaffold only:
the current Ansible Vault files remain in place and are not converted in this
phase.

## Architecture

SOPS encrypts each secret file with a per-file data key. That data key is
wrapped for every recipient selected by `.sops.yaml`. The repo commits the
encrypted file and SOPS metadata, not private keys.

Phase 1 keeps the model simple:

- `.sops.yaml` is committed at the repo root with invalid placeholder
  recipients. The next ceremony step is replacing those placeholders with real
  public age/YubiKey recipients.
- `.sops.yaml.example` remains as a reference copy of the same path policy.
- Existing Ansible Vault files continue to serve deploy workflows until the
  migration phase changes playbooks and helpers.
- `community.sops` is available for the later Ansible integration phase.

## Recipient Groups

Routine deploy files use the non-root/service decrypt group plus deploy CI:

- `penso`
- `larry`
- `rhaki`
- `zexsor`
- `j0nl1`
- `kyar1s`
- `deploy-ci`

Root/debian files use the root/debian group only:

- `penso`
- `larry`
- `j0nl1`

Team decisions:

- Each user gets one YubiKey-backed age recipient.
- Users do not get backup age keys in this repo.
- `deploy-ci` is generated on a trusted desktop, not on a remote server.
- In phase 1, `deploy-ci` is included only for routine deploy files and is not
  included for root/debian files.
- `key_groups` are deferred to phase 2.

## Planned SOPS Files

These future files mirror the 11 current Vault-backed classes.

| Current Vault file | Future SOPS file | Recipients |
| --- | --- | --- |
| `group_vars/all/vault.yml` | `group_vars/all/vault.sops.yml` | routine |
| `group_vars/all/deploy_key.vault` | `group_vars/all/deploy_key.sops` | routine |
| `group_vars/dango-assistant/vault.yml` | `group_vars/dango-assistant/vault.sops.yml` | routine |
| `group_vars/hyperlane/vault.yml` | `group_vars/hyperlane/vault.sops.yml` | routine |
| `group_vars/perps-bot/vault.yml` | `group_vars/perps-bot/vault.sops.yml` | routine |
| `group_vars/points-bot/vault.yml` | `group_vars/points-bot/vault.sops.yml` | routine |
| `host_vars/100.96.253.40/vault.yml` | `host_vars/100.96.253.40/vault.sops.yml` | routine |
| `host_vars/100.107.248.71/vault.yml` | `host_vars/100.107.248.71/vault.sops.yml` | routine |
| `host_vars/100.122.37.57/main.yml` | `host_vars/100.122.37.57/main.sops.yml` | routine |
| `vaults/debian/debian_key.vault` | `vaults/debian/debian_key.sops` | root/debian |
| `vaults/debian/root_vault.yml` | `vaults/debian/root_vault.sops.yml` | root/debian |

## Install And Setup

Install local tools:

```bash
brew install sops age
brew install age-plugin-yubikey
```

Check the local setup without decrypting anything:

```bash
cd deploy
just sops-check
just sops-audit
```

Replace placeholders only after recipient collection:

```bash
cd ..
$EDITOR .sops.yaml
```

Commit the updated `.sops.yaml` only after the team has reviewed the real public
recipients. The public recipient strings are safe to commit; local identity
files and private age keys are not.

## Recipient Collection

Each person creates one YubiKey-backed age recipient locally. A typical flow is:

```bash
age-plugin-yubikey --generate
age-plugin-yubikey --list
```

Then collect the public recipient only:

```bash
cd deploy
just sops-collect-recipient --user j0nl1 --group routine age1...
```

The helper validates that the value looks like a public age recipient and prints
a reviewable snippet. It does not edit `.sops.yaml`.

## CI Age Key Creation

Generate `deploy-ci` on a trusted desktop with local disk permissions locked
down:

```bash
umask 077
age-keygen -o deploy-ci.agekey
age-keygen -y deploy-ci.agekey
```

Use the public recipient from `age-keygen -y` in routine deploy rules only. Put
the private key into the CI secret store using the deployment platform's normal
secret mechanism. Do not include `deploy-ci` in root/debian rules in phase 1.
For GitHub Actions, store it in the `deploy` environment as `SOPS_AGE_KEY` and
pass it as both `SOPS_AGE_KEY` and `ANSIBLE_SOPS_AGE_KEY` when wiring SOPS into
Ansible.

## Daily Usage

During the transition, existing Vault recipes stay available:

```bash
cd deploy
just edit-secrets
just edit-root-secrets
```

Future SOPS edit recipes are staged now and will work once the real `.sops.yaml`
and encrypted files exist:

```bash
cd deploy
just sops-edit-secrets
just sops-edit-hyperlane-secrets
just sops-edit-root-secrets
```

The audit helper is metadata-only. It does not call `sops --decrypt` and does
not print encrypted values:

```bash
cd deploy
just sops-audit
```

## Re-Encryption

After adding or removing a public recipient from `.sops.yaml`, update existing
SOPS files:

```bash
cd deploy
just sops-reencrypt --dry-run
just sops-reencrypt
```

The helper fails if `.sops.yaml` or `sops` is missing. It only processes the
planned future `*.sops.yml` or `*.sops` paths and refuses unexpected paths.

## Revocation Caveats

Removing a recipient from `.sops.yaml` does not undo access to old Git history
or to plaintext a person already decrypted. It only prevents that recipient from
decrypting files after the files are re-encrypted and the updated ciphertext is
merged.

For meaningful revocation:

- remove the public recipient from `.sops.yaml`
- run `just sops-reencrypt`
- rotate affected service credentials when access exposure matters
- remove or rotate the corresponding CI secret when revoking `deploy-ci`
- treat root/debian material as requiring credential rotation, not only SOPS
  metadata updates

## Phase 1 Vs Phase 2 `key_groups`

Phase 1 uses one recipient list per path rule. This is easier to audit while the
team is moving from Ansible Vault to SOPS.

Phase 2 can move to SOPS `key_groups` when the team wants quorum-style decrypt
policy. For example, root/debian files could require separate groups for root
operators and an offline break-glass process. That change should be made only
after the phase 1 migration is stable, because `key_groups` changes the decrypt
policy semantics and must be tested carefully.

## Migration Plan

1. Keep current Ansible Vault files and recipes working.
2. Collect public YubiKey recipients from the six routine users and the three
   root/debian users.
3. Generate the `deploy-ci` age key on a trusted desktop and store only its
   public recipient in `.sops.yaml`.
4. Replace placeholders in the committed `.sops.yaml`.
5. Convert one non-root Vault file to its matching `.sops.yml` file in a small
   reviewable change.
6. Add Ansible loading for that SOPS file using `community.sops`.
7. Repeat for the remaining routine files.
8. Convert root/debian files separately, without `deploy-ci` in phase 1.
9. Remove legacy Vault helpers only after all deploy playbooks are using SOPS
   and the team has verified rollback expectations.
