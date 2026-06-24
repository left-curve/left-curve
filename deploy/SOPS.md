# SOPS deploy secrets

This repository stores deploy secrets with SOPS and age recipients. Human
recipients are YubiKey-backed through `age-plugin-yubikey`; the CI recipient is
an age key generated on a trusted desktop and stored only in GitHub secrets.

## Architecture

SOPS encrypts each file with a random data key. That data key is wrapped for the
age recipients selected by `.sops.yaml`. Public recipient strings are safe to
commit. Private key material stays either in a teammate's YubiKey or in the
GitHub `deploy` environment secret `SOPS_AGE_KEY`.

Ansible integration uses:

- `community.sops.sops` as the `group_vars`/`host_vars` vars plugin.
- `community.sops.load_vars` for root/debian files that must not be decrypted
  by deploy CI.
- `SOPS_AGE_KEY` / `ANSIBLE_SOPS_AGE_KEY` in GitHub Actions.

## Recipient Groups

Routine deploy files use the non-root/service decrypt group plus `github-ci`:

- `penso`
- `larry`
- `rhaki`
- `zexsor`
- `j0nl1`
- `kyar1s`
- `github-ci`

Root/debian files use:

- `penso`
- `larry`
- `j0nl1`

Team decisions:

- Each user gets one YubiKey-backed age recipient.
- Users do not get backup age keys in this repo.
- `github-ci` is generated on a trusted desktop, not on a remote server.
- In phase 1, `github-ci` is included only for routine deploy files and is not
  included for root/debian files.
- If a user loses a key, add the new public recipient and re-encrypt.
- Hyperlane stays in `group_vars/hyperlane/vault.sops.json` for this cutover.
  The per-validator custody split from PR #2170 is deferred.
- SOPS `key_groups` are deferred to phase 2.

## SOPS Files

| Legacy Vault file | SOPS file | Recipients |
| --- | --- | --- |
| `group_vars/all/vault.yml` | `group_vars/all/vault.sops.json` | routine |
| `group_vars/all/deploy_key.vault` | `vaults/deploy/deploy_key.sops` | routine |
| `group_vars/dango-assistant/vault.yml` | `group_vars/dango-assistant/vault.sops.json` | routine |
| `group_vars/hyperlane/vault.yml` | `group_vars/hyperlane/vault.sops.json` | routine |
| `group_vars/perps-bot/vault.yml` | `group_vars/perps-bot/vault.sops.json` | routine |
| `group_vars/points-bot/vault.yml` | `group_vars/points-bot/vault.sops.json` | routine |
| `host_vars/100.96.253.40/vault.yml` | `host_vars/100.96.253.40/vault.sops.json` | routine |
| `host_vars/100.107.248.71/vault.yml` | `host_vars/100.107.248.71/vault.sops.json` | routine |
| `host_vars/100.122.37.57/main.yml` | `host_vars/100.122.37.57/main.sops.json` | routine |
| `vaults/debian/debian_key.vault` | `vaults/debian/debian_key.sops` | root/debian |
| `vaults/debian/root_vault.yml` | `vaults/debian/root_vault.sops.json` | root/debian |

## File Format

Vault files are stored as SOPS JSON (`*.sops.json`); the binary SSH-key files
(`*.sops`) are unaffected. Do not create a `.sops.yml` vault.

JSON is required, not cosmetic. SOPS' YAML output emits `0x`-prefixed hex
strings (such as EVM and validator addresses) unquoted, because they exceed
int64 and Go's YAML reader treats them as plain scalars. Ansible's YAML loader
(PyYAML) then reads that unquoted `0x...` token as an integer, silently changing
the value the deploy receives. JSON quotes every string, so each value
round-trips with its intended type, and a future `0x` value cannot reintroduce
the bug.

JSON has no comment syntax, so any annotations in a vault are dropped on
encryption; put labels in a data field instead of a comment.

## Local Setup

Install local tools:

```bash
brew install sops age age-plugin-yubikey
```

Create or list a YubiKey recipient:

```bash
age-plugin-yubikey --generate
age-plugin-yubikey --list
```

Store the local identity file outside the repository. Commit only the public
`age1yubikey1...` recipient in `.sops.yaml`.

Check local tooling without decrypting secrets:

```bash
cd deploy
just sops-check
just sops-audit
```

## Deploy CI Key

Generate `github-ci` on a trusted desktop with local disk permissions locked
down:

```bash
umask 077
age-keygen -o github-ci.agekey
age-keygen -y github-ci.agekey
```

Use the public recipient from `age-keygen -y` in routine deploy rules only. Put
the private key into the CI secret store using the deployment platform's normal
secret mechanism. Do not include `github-ci` in root/debian rules in phase 1.
For GitHub Actions, store it in the `deploy` environment as `SOPS_AGE_KEY` and
pass it as both `SOPS_AGE_KEY` and `ANSIBLE_SOPS_AGE_KEY` when wiring SOPS into
Ansible.

## Migration Ceremony

Run the migration on a trusted machine that still has access to the legacy
default and debian Vault passwords.

The ceremony should:

- confirm the `github-ci` public recipient in `.sops.yaml`
- store the `github-ci` private age key in the GitHub `deploy` environment as
  `SOPS_AGE_KEY`
- decrypt each legacy Vault file locally
- write the matching SOPS file using the recipient rules in `.sops.yaml`
- delete the legacy Vault files and Vault password helper scripts
- verify that no legacy Vault payloads remain under `deploy/`
- verify that GitHub runners have `/usr/local/bin/sops`

After the ceremony:

```bash
cd deploy
just sops-audit
```

Check for old Vault references:

```bash
rg -n 'vault_password_file|vault_identity_list|ANSIBLE_VAULT_PASSWORD|ANSIBLE_DEBIAN_PASSWORD|root_vault\.yml|\.vault' \
  deploy .github/workflows \
  --glob '!deploy/SOPS.md'
```

## Daily Usage

Edit routine secrets:

```bash
cd deploy
just sops-edit-secrets
just sops-edit-hyperlane-secrets
just sops-edit-points-bot-secrets
just sops-edit-perps-bot-secrets
just sops-edit-dango-assistant-secrets
```

Edit root/debian secrets:

```bash
cd deploy
just sops-edit-root-secrets
just sops-edit-root-key
```

Load SSH keys into the local agent:

```bash
cd deploy
just add-deploy-key
just add-debian-key
```

## Re-Encryption

After adding or removing a public recipient from `.sops.yaml`, update existing
SOPS files:

```bash
cd deploy
just sops-reencrypt --dry-run
just sops-reencrypt
```

Removing a recipient protects future encrypted versions only.

## Revocation

Removing a recipient from `.sops.yaml` does not undo access to old Git history
or to plaintext a person already decrypted. It only prevents that recipient from
decrypting files after the files are re-encrypted and the updated ciphertext is
merged.

For meaningful revocation:

- remove the public recipient from `.sops.yaml`
- run `just sops-reencrypt`
- rotate affected service credentials when access exposure matters
- remove or rotate the corresponding CI secret when revoking `github-ci`
- treat root/debian material as requiring credential rotation, not only SOPS
  metadata updates

## Phase 1 Vs Phase 2 `key_groups`

Phase 1 uses one recipient list per path rule. This is easier to audit while the
team is moving from Ansible Vault to SOPS.

Phase 2 can move root/debian or other high-value files to SOPS `key_groups` with
a `shamir_threshold`, for example 2-of-N decrypt. That should stay out of
routine deploy paths because one SOPS process needs access to enough groups at
decrypt time.

## Migration Plan

1. Confirm all committed public recipients in `.sops.yaml`.
2. Confirm the GitHub `deploy` environment has `SOPS_AGE_KEY`.
3. Convert each legacy Vault file to its matching `.sops.json` or `.sops` file.
4. Remove the legacy Vault files in the same change that adds their SOPS
   replacements.
5. Run `just sops-audit` from `deploy/`.
6. Run the normal deploy workflow only after the audit passes.
