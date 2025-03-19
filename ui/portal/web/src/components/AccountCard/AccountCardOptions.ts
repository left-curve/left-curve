import { AccountType } from "@left-curve/dango/types";

export const AccountCardOptions = {
  [AccountType.Spot]: {
    text: "Spot",
    badge: "blue",
    bgColor: "bg-account-card-red",
    img: "/images/characters/dog.svg",
    imgClassName: "opacity-60 right-[-2.9rem] bottom-[-4.3rem] scale-x-[-1] w-[14rem]",
  },
  [AccountType.Multi]: {
    text: "Multisig",
    badge: "green",
    bgColor: "bg-account-card-blue",
    img: "/images/characters/puppy.svg",
    imgClassName: "opacity-50 right-[-1rem] bottom-[-4.3rem] w-[15.4rem]",
  },
  [AccountType.Margin]: {
    text: "Margin",
    badge: "red",
    bgColor: "bg-account-card-green",
    img: "/images/characters/froggo.svg",
    imgClassName: "opacity-60 w-[15rem] bottom-[-5rem] right-[-0.5rem]",
  },
} as const;
