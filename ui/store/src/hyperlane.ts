import { sepolia, mainnet, base, arbitrum } from "viem/chains";

export const chains = {
  sepolia: {
    ...sepolia,
    contracts: {
      ...sepolia.contracts,
      erc20: [
        {
          symbol: "USDC",
          address: "0x1c7d4b196cb0c7b01d743fbc6116a902379c7238",
          decimals: 6,
          targetDenom: "bridge/usdc",
        },
      ],
    },
  },
  ethereum: {
    ...mainnet,
    contracts: {
      ...mainnet.contracts,
      erc20: [
        {
          symbol: "USDT",
          address: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
          decimals: 6,
          targetDenom: "bridge/usdt",
        },
        {
          symbol: "USDC",
          address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
          decimals: 6,
          targetDenom: "bridge/usdc",
        },
      ],
    },
  },
  base: {
    ...base,
    contracts: {
      ...base.contracts,
      erc20: [
        {
          symbol: "USDC",
          address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
          decimals: 6,
          targetDenom: "bridge/usdc",
        },
        {
          symbol: "WETH",
          address: "0x4200000000000000000000000000000000000006",
          decimals: 18,
          targetDenom: "bridge/weth",
        },
      ],
    },
  },
  arbitrum: {
    ...arbitrum,
    contracts: {
      ...arbitrum.contracts,
      erc20: [
        {
          symbol: "USDT",
          address: "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
          decimals: 6,
          targetDenom: "bridge/usdt",
        },
        {
          symbol: "USDC",
          address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
          decimals: 6,
          targetDenom: "bridge/usdc",
        },
      ],
    },
  },
};
