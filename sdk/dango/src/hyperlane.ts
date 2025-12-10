export const DANGO_DOMAIN = 88888888;

export {
  HYPERLANE_DOMAIN_KEY,
  MAILBOX_VERSION,
  Addr32,
  IncrementalMerkleTree,
  Message,
  Metadata,
  TokenMessage,
  mockValidatorSet,
  mockValidatorSign,
} from "@left-curve/sdk/hyperlane";

export type { ValidatorSet, Domain } from "@left-curve/sdk/hyperlane";

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
