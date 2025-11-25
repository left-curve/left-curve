# Dango Hyperlane Deployment

This crate contains the code for deploying Hyperlane Warp Routes on EVM and connecting them to a Dango network.

## EVM

The `hyperlane-monorepo` is included as git sumbodule in `dango/hyperlane-deployment/hyperlane-monorepo. All the binaries we deploy are built from this source. To rebuild the EVM contracts from source, run the following.

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
