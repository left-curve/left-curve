export const HYPERLANE_ROUTER_ABI = [
  {
    inputs: [
      { name: "_destination", type: "uint32" },
      { name: "_recipient", type: "bytes32" },
      { name: "_amountOrId", type: "uint256" },
    ],
    name: "transferRemote",
    outputs: [{ name: "", type: "bytes32" }],
    stateMutability: "payable",
    type: "function",
  },
] as const;

export const INFURA_URLS = {
  "1": "https://mainnet.infura.io/v3/00f81bbb13ef4da997f6351b8146807e",
  "11155111": "https://sepolia.infura.io/v3/2de96f6db6d34eccaa8935cabb9b29c8",
  "8453": "base",
  "42161": "arbitrum",
};

export const ERC20_ABI = [
  {
    inputs: [
      { name: "spender", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    name: "approve",
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "nonpayable",
    type: "function",
  },
  {
    type: "function",
    name: "balanceOf",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    inputs: [
      { name: "owner", type: "address" },
      { name: "spender", type: "address" },
    ],
    name: "allowance",
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
    type: "function",
  },
] as const;
