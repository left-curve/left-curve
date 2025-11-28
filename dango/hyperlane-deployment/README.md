# Dango Hyperlane Deployment

This crate contains the code for deploying Hyperlane Warp Routes on EVM and connecting them to a Dango network.

## EVM

The `hyperlane-monorepo` is included as git sumbodule in `dango/hyperlane-deployment/hyperlane-monorepo. All the binaries we deploy are built from this source.
The binaries used are located in the `dango/hyperlane-deployment/artifacts/evm` directory. If you don't need to rebuild the contracts, you can skip this section.

### Rebuilding the Solidity artifacts

To rebuild the EVM contracts from source, run the following.

1. Checkout the new desired commit of the `hyperlane-monorepo`

2. Build the EVM contracts

```bash
cd dango/hyperlane-deployment/hyperlane-monorepo
yarn install
yarn build
```

3. Copy the Solidity artifacts to the `dango/hyperlane-deployment` directory

```bash
cp ./hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/ProxyAdmin.json artifacts/evm
cp ./hyperlane-monorepo/node_modules/@arbitrum/token-bridge-contracts/node_modules/@openzeppelin/contracts/build/contracts/TransparentUpgradeableProxy.json artifacts/evm
cp ./hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20Collateral.sol/HypERC20Collateral.json artifacts/evm
cp ./hyperlane-monorepo/solidity/artifacts/contracts/token/HypERC20.sol/HypERC20.json artifacts/evm
cp ./hyperlane-monorepo/solidity/artifacts/contracts/token/HypNative.sol/HypNative.json artifacts/evm
```

### Running the deployment scripts

Before you can run the deployment scripts make sure you have the correct environment variables set. See the EVM section of `dango/hyperlane-deployment/env.example`.

The deployment scripts are located in the `dango/hyperlane-deployment/src/bin/evm/` directory. Use the `dango/hyperlane-deployment/config.json` file to configure the deployment.
The main script is `deploy.rs`. It deploys the Hyperlane Routes that are configured in the config file.

## SVM

### Deploying on SVM

The binaries for the SVM deployment are also built from the `hyperlane-monorepo`. To rebuild the SVM contracts from source, first make sure you have the right version of the Solana tooling. Currently,

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/v1.18.18/install)"
```

but refer to the [Hyperland Docs](https://docs.hyperlane.xyz/docs/guides/warp-routes/svm/svm-warp-route-guide#step-2:-prepare-for-deployment) for the latest version. Verify your solana version with `solana --version`.

Once you have the right version of the Solana tooling, build the SVM contracts from source.

```bash
cd dango/hyperlane-deployment/hyperlane-monorepo/rust/sealevel/programs
./build-programs.sh token
```

3. Copy the SVM artifacts to the `dango/hyperlane-deployment` directory

```bash
cp ./hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token_native.so artifacts/svm
cp ./hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token_collateral.so artifacts/svm
cp ./hyperlane-monorepo/rust/sealevel/target/deploy/hyperlane_sealevel_token.so artifacts/svm
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

### Flow
