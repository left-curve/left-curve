# Dango Hyperlane Deployment

This crate contains the code for deploying Hyperlane Warp Routes on EVM and connecting them to a Dango network.

## EVM

The `hyperlane-monorepo` is included as git sumbodule in `dango/hyperlane-deployment/hyperlane-monorepo. All the binaries we deploy are built from this source.

The binaries used are located in the `dango/hyperlane-deployment/artifacts/evm` directory. If you don't need to rebuild the contracts, you can skip this section.

### Rebuilding the Solidity artifacts

To rebuild the EVM contracts from source, run the following.

1. Clone the [left-curve fork of hyperlane-monorepo](https://github.com/left-curve/hyperlane-monorepo/tree/dango) and check out to the desired commit. Typically this should be the `dango` branch:

   ```bash
   git clone https://github.com/left-curve/hyperlane-monorepo.git
   cd hyperlane-monorepo
   git checkout dango
   ```

2. Build the EVM contracts:

   ```bash
   yarn install
   yarn build
   ```

3. Back to the left-curve repo. Copy the Solidity artifacts to the `dango/hyperlane-deployment` directory:

   ```bash
   for f in \
     /path/to/hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/ProxyAdmin.json \
     /path/to/hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/TransparentUpgradeableProxy.json \
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20Collateral.sol/HypERC20Collateral.json \
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/HypERC20.json \
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypNative.sol/HypNative.json
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/StaticMessageIdMultisigIsm.json \
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/StaticMessageIsMultisigIsmFactory.json \
     /path/to/hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/TokenRouter.json
   do
     cp "$f" artifacts/evm
   done
   ```

### Running the deployment scripts

Before you can run the deployment scripts make sure you have the correct environment variables set. See the EVM section of `dango/hyperlane-deployment/env.example`.

The deployment scripts are located in the `dango/hyperlane-deployment/src/bin/evm/` directory. Use the `dango/hyperlane-deployment/config.json` file to configure the deployment.
The main script is `deploy.rs`. It deploys the Hyperlane Routes that are configured in the config file.

To deploy the EVM contracts, run the following:

```bash
cargo run -p dango-hyperlane-deployment --bin evm-deploy -- --config config.json --deployments deployments.json
```

### Build artifacts

| Contract                    | Repository                                  | Commit    | Solc   | ethVersion | Optimizer             |
| --------------------------- | ------------------------------------------- | --------- | ------ | ---------- | --------------------- |
| ProxyAdmin                  | [@openzeppelin/contracts][oz-repo]          | `0a25c19` | 0.8.13 | `london`   | enabled; 200 runs     |
| TransparentUpgradeableProxy | [@openzeppelin/contracts][oz-repo]          | `0a25c19` | 0.8.13 | `london`   | enabled; 200 runs     |
| HypNative                   | [hyperlane-xyz/hyperlane-monorepo][hl-repo] | `e657bbc` | 0.8.22 | `paris`    | enabled; 999,999 runs |
| HypERC20Collateral          | [hyperlane-xyz/hyperlane-monorepo][hl-repo] | `e657bbc` | 0.8.22 | `paris`    | enabled; 999,999 runs |
| StaticMessageIdMultisigIsm  | [hyperlane-xyz/hyperlane-monorepo][hl-repo] | `e657bbc` | 0.8.22 | `paris`    | enabled; 999,999 runs |

[oz-repo]: https://github.com/OpenZeppelin/openzeppelin-contracts/tree/0a25c1940ca220686588c4af3ec526f725fe2582
[hl-repo]: https://github.com/hyperlane-xyz/hyperlane-monorepo/tree/e657bbce607e39017add3f68be6e4cd6850981b8

### Addresses

#### Ethereum Mainnet (chain ID: 1)

| Contract                    | Asset | Address                                                                                                                 |
| --------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------- |
| ProxyAdmin                  | -     | [`0x613942eff27c6886bb2a33a172cdaf03a009e601`](https://etherscan.io/address/0x613942eff27c6886bb2a33a172cdaf03a009e601) |
| TransparentUpgradeableProxy | ETH   | [`0x9d259aa1ec7324c7433b89d2935b08c30f3154cb`](https://etherscan.io/address/0x9d259aa1ec7324c7433b89d2935b08c30f3154cb) |
| HypNative                   | ETH   | [`0x9d0ea335355da17ee89e50df43ab823416cf73d4`](https://etherscan.io/address/0x9d0ea335355da17ee89e50df43ab823416cf73d4) |
| TransparentUpgradeableProxy | USDC  | [`0xd05909852ae07118857f9d071781671d12c0f36c`](https://etherscan.io/address/0xd05909852ae07118857f9d071781671d12c0f36c) |
| HypERC20Collateral          | USDC  | [`0xe071653043828c9923c79b04b077358d94fc84f9`](https://etherscan.io/address/0xe071653043828c9923c79b04b077358d94fc84f9) |
| StaticMessageIdMultisigIsm  | -     | [`0x17972F088Ad3e10C3E15E4960f8547230362C57E`](https://etherscan.io/address/0x17972F088Ad3e10C3E15E4960f8547230362C57E) |

#### Sepolia (chain ID: 11155111)

| Contract                    | Asset | Address                                                                                                                         |
| --------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------- |
| ProxyAdmin                  | -     | [`0x59cf4f33ce42afa957b93e68031f07bf6d299d60`](https://sepolia.etherscan.io/address/0x59cf4f33ce42afa957b93e68031f07bf6d299d60) |
| TransparentUpgradeableProxy | ETH   | [`0xe3109f83bef36aece35870ee1b2e07a5dd12cfa9`](https://sepolia.etherscan.io/address/0xe3109f83bef36aece35870ee1b2e07a5dd12cfa9) |
| HypNative                   | ETH   | [`0xb4513d39e6839bf7c1f01a65e294bab8b16b5887`](https://sepolia.etherscan.io/address/0xb4513d39e6839bf7c1f01a65e294bab8b16b5887) |
| TransparentUpgradeableProxy | USDC  | [`0x0d8c3516df20cff940e479ea2d8c7d1dd0a706ac`](https://sepolia.etherscan.io/address/0x0d8c3516df20cff940e479ea2d8c7d1dd0a706ac) |
| HypERC20Collateral          | USDC  | [`0x26bc0e68467d88cedb5a3793618c8f6586512706`](https://sepolia.etherscan.io/address/0x26bc0e68467d88cedb5a3793618c8f6586512706) |
| StaticMessageIdMultisigIsm  | -     | [`0x08A587C17C1CD3a1BC2220E0808281a143877B70`](https://sepolia.etherscan.io/address/0x08A587C17C1CD3a1BC2220E0808281a143877B70) |

#### Arbitrum One (chain ID: 42161)

| Contract                    | Asset | Address |
| --------------------------- | ----- | ------- |
| ProxyAdmin                  | -     | TBD     |
| TransparentUpgradeableProxy | ETH   | TBD     |
| HypNative                   | ETH   | TBD     |
| TransparentUpgradeableProxy | USDC  | TBD     |
| HypERC20Collateral          | USDC  | TBD     |
| StaticMessageIdMultisigIsm  | -     | TBD     |

#### Arbitrum Sepolia (chain ID: 421614)

| Contract                    | Asset | Address |
| --------------------------- | ----- | ------- |
| ProxyAdmin                  | -     | TBD     |
| TransparentUpgradeableProxy | ETH   | TBD     |
| HypNative                   | ETH   | TBD     |
| TransparentUpgradeableProxy | USDC  | TBD     |
| HypERC20Collateral          | USDC  | TBD     |
| StaticMessageIdMultisigIsm  | -     | TBD     |

## SVM

### Deploying on SVM

1. The binaries for the SVM deployment are also built from the `hyperlane-monorepo`. To rebuild the SVM contracts from source, first make sure you have the right version of the Solana tooling. Currently,

   ```bash
   sh -c "$(curl -sSfL https://release.anza.xyz/v1.18.18/install)"
   ```

   but refer to the [Hyperland Docs](https://docs.hyperlane.xyz/docs/guides/warp-routes/svm/svm-warp-route-guide#step-2:-prepare-for-deployment) for the latest version. Verify your solana version with `solana --version`.

2. Once you have the right version of the Solana tooling, build the SVM contracts from source.

   ```bash
   cd /path/to/left-curve/hyperlane-monorepo/rust/sealevel/programs
   ./build-programs.sh token
   ```

3. Copy the SVM artifacts to the `dango/hyperlane-deployment` directory

   ```bash
   for f in \
     /path/to/hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token_native.so \
     /path/to/hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token_collateral.so \
     /path/to/hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token.so
   do
     cp "$f" artifacts/svm
   done
   ```

4. To deploy we will use the Solana CLI. Generate a keypair from and fund it on the relevant chain.

   ```bash
   solana-keygen new --outfile keypair.json
   ```

5. Now we can deploy the HWR.

   ```bash
   solana program deploy ./hyperlane_sealevel_token_native.so --keypair keypair.json
   solana program deploy ./hyperlane_sealevel_token_collateral.so --keypair keypair.json
   solana program deploy ./hyperlane_sealevel_token.so --keypair keypair.json
   ```

6. Now we can configure the IGP.

   ```bash
   solana program invoke --program-id hBHAApi5ZoeCYHqDdCKkCzVKmBdwywdT3hMqe327eZB --keypair keypair.json --instruction configure-igp
   ```
