# Hyperlane SOPS secrets

This directory holds the SOPS-encrypted Hyperlane credentials, one file per
agent so decrypt access can differ per file (see `deploy/SOPS.md`, section
"Hyperlane validator key custody", and the rules in the repo-root
`.sops.yaml`):

- `mainnet-validator-{1..4}.sops.yml` — custody-restricted: 3 custodians per
  validator, no `deploy-ci`.
- `testnet-validator-{1,2}.sops.yml` — routine group + `deploy-ci`.
- `relayer.sops.yml` — routine group + `deploy-ci`; holds the
  `relayer: { mainnet: ..., testnet: ... }` dict.

The files are created during the custody cutover (see "Hyperlane custody
cutover" in `deploy/SOPS.md`); until then only the plaintext templates
`validator.sops.yml.example` and `relayer.sops.yml.example` live here — copy
one, fill it in, and `sops --encrypt --in-place` it as described in the
template headers. Edit existing files with:

```bash
cd deploy
just sops-edit-hyperlane-validator mainnet 1
just sops-edit-hyperlane-relayer
```
