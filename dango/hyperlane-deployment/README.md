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
