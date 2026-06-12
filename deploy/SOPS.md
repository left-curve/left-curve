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

Hyperlane mainnet validator files use per-validator custody groups instead of
the routine group. See "Hyperlane validator key custody" below.

Team decisions:

- Each user gets one YubiKey-backed age recipient.
- Users do not get backup age keys in this repo.
- `deploy-ci` is generated on a trusted desktop, not on a remote server.
- In phase 1, `deploy-ci` is included only for routine deploy files and is not
  included for root/debian files.
- `deploy-ci` is excluded from hyperlane mainnet validator files: mainnet
  validator deploys are manual and YubiKey-touch-gated.
- `key_groups` are deferred to phase 2.

## Hyperlane Validator Key Custody

The 4 mainnet Hyperlane validator signing keys live in AWS KMS; what the repo
stores is the IAM credentials that authorize signing with them. Under the
shared Ansible Vault password, every member effectively controlled all 4 keys
— enough to single-handedly meet the 3-of-4 ISM threshold and forge
cross-chain messages. Custody splits that control.

Requirements:

- No single member controls more than 2 of the 4 keys (2 < 3-of-4 threshold).
- Any 2 members together control at least 3 keys, so any two available
  members can operate a validator quorum.

These constraints force exactly one structure: every member holds exactly 2
keys, and all 6 members hold distinct key pairs — the 6 possible pairs of 4
keys. Each key therefore has exactly 3 custodians.

| Key | Custodians |
| --- | --- |
| mainnet validator-1 | penso, larry, rhaki |
| mainnet validator-2 | penso, zexsor, j0nl1 |
| mainnet validator-3 | larry, zexsor, kyar1s |
| mainnet validator-4 | rhaki, j0nl1, kyar1s |

Who holds which pair is swappable as long as all 6 pairs stay distinct; the
`.sops.yaml` rules are the source of truth.

Properties and accepted trade-offs:

- Single-member safety: no member can reach the ISM threshold alone.
- Availability: 3 custodians per key means even two lost YubiKeys leave at
  least one custodian per key, and any single member being unavailable never
  blocks a deploy.
- No pair-collusion resistance, by design: "any 2 members cover >= 3 keys"
  mathematically guarantees that every pair of members can jointly meet the
  3-of-4 threshold. Maximal availability and pair-collusion resistance are
  incompatible with 4 keys; the team chose availability.
- The custody gate applies at decrypt time (deploys, edits), not per
  signature — validators sign checkpoints continuously and unattended with
  the KMS keys.

Known limitation (out of scope here): the decrypted credentials land in
plaintext `.env` files on the validator hosts, and the deploy/debian SSH keys
are shared by the whole team — anyone with host access still controls all 4
keys regardless of SOPS custody. hetzner3 even co-hosts mainnet validator-4
and the mainnet relayer. Real end-to-end custody needs per-person SSH keys
authorized per host to match the custodian matrix; that is a separate effort.
AWS-side hardening (scoping each KMS key policy to its own IAM user,
CloudTrail alerts on `kms:Sign` from unexpected principals) is also tracked
outside this repo.

## Planned SOPS Files

These future files mirror the 11 current Vault-backed classes.

| Current Vault file | Future SOPS file | Recipients |
| --- | --- | --- |
| `group_vars/all/vault.yml` | `group_vars/all/vault.sops.yml` | routine |
| `group_vars/all/deploy_key.vault` | `group_vars/all/deploy_key.sops` | routine |
| `group_vars/dango-assistant/vault.yml` | `group_vars/dango-assistant/vault.sops.yml` | routine |
| `group_vars/hyperlane/vault.yml` | `vaults/hyperlane/mainnet-validator-{1..4}.sops.yml` | per-validator custodians |
| (same vault file) | `vaults/hyperlane/testnet-validator-{1,2}.sops.yml` | routine |
| (same vault file) | `vaults/hyperlane/relayer.sops.yml` | routine |
| `group_vars/perps-bot/vault.yml` | `group_vars/perps-bot/vault.sops.yml` | routine |
| `group_vars/points-bot/vault.yml` | `group_vars/points-bot/vault.sops.yml` | routine |
| `host_vars/100.96.253.40/vault.yml` | `host_vars/100.96.253.40/vault.sops.yml` | routine |
| `host_vars/100.107.248.71/vault.yml` | `host_vars/100.107.248.71/vault.sops.yml` | routine |
| `host_vars/100.122.37.57/main.yml` | `host_vars/100.122.37.57/main.sops.yml` | routine |
| `vaults/debian/debian_key.vault` | `vaults/debian/debian_key.sops` | root/debian |
| `vaults/debian/root_vault.yml` | `vaults/debian/root_vault.sops.yml` | root/debian |

The hyperlane files intentionally live in `vaults/hyperlane/`, not
`group_vars/`. The `community.sops` vars plugin decrypts every `*.sops.yml`
under `group_vars/` for any play touching the group and fails when the runner
is not a recipient — so custody-split files there would break every hyperlane
play for every non-custodian. Instead, `hyperlane.yml` and
`hyperlane-kms-address.yml` load only the single needed file with
`community.sops.load_vars`, selected by network/agent/index. A side benefit is
that secret-free plays like `stop-hyperlane.yml` no longer touch any
encrypted material.

Each hyperlane SOPS file keeps the nested shape the role already expects, so
`roles/hyperlane` is unchanged. For example
`vaults/hyperlane/mainnet-validator-1.sops.yml`:

```yaml
validator:
  mainnet:
    "1":
      aws_access_key_id: ...
      aws_secret_access_key: ...
      aws_kms_alias: ...
      checkpoints_bucket: ...
      dango_signer_key: ...
      dango_signer_address: ...
```

`vaults/hyperlane/relayer.sops.yml` holds the existing
`relayer: { mainnet: ..., testnet: ... }` dict.

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
just sops-edit-hyperlane-validator mainnet 1
just sops-edit-hyperlane-relayer
just sops-edit-root-secrets
```

Editing `vaults/hyperlane/mainnet-validator-N.sops.yml` works only for that
validator's custodians and requires a YubiKey touch.

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

Note that `sops updatekeys` must decrypt the file's data key, so re-encrypting
a custody-restricted hyperlane file only works for one of its custodians. A
full `just sops-reencrypt` run by a single person will fail on the custody
files they cannot decrypt; pass explicit paths and have each validator's
custodian re-encrypt their own files.

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

### Hyperlane custody cutover

The hyperlane wiring in `hyperlane.yml` and `hyperlane-kms-address.yml` reads
the `vaults/hyperlane/*.sops.yml` files unconditionally, so this branch must
not merge before the cutover is complete:

1. All placeholders in `.sops.yaml` are replaced with real recipients (j0nl1,
   kyar1s, deploy-ci).
2. Anyone creates the 7 `vaults/hyperlane/` files with placeholder values —
   encryption only needs the public recipients. Start from
   `vaults/hyperlane/validator.sops.yml.example` and
   `vaults/hyperlane/relayer.sops.yml.example`, which document every field.
3. Each mainnet validator's custodians rotate that validator's IAM access
   keys in AWS and `sops`-edit the real values in. The old IAM keys sit in
   Git history under the shared Vault password and must be treated as burned.
   Non-rotating fields (`aws_kms_alias`, `checkpoints_bucket`,
   `dango_signer_address`) can be copied from the old vault; rotate
   `dango_signer_key` as well.
4. Relayer and testnet credentials are rotated and populated the same way
   (recipients: routine group + deploy-ci).
5. `group_vars/hyperlane/vault.yml` is deleted (done on this branch) and the
   team coordinates so no hyperlane deploy lands between rotation and merge.
6. Follow-up outside this repo: scope each KMS key policy to its own IAM
   user; add CloudTrail alerts on `kms:Sign` from unexpected principals.

After the cutover, CI can no longer decrypt mainnet validator credentials —
mainnet validator deploys are deliberately manual.
