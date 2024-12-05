import type React from "react";
import {
  Ants,
  Factory1,
  Factory2,
  Fisher,
  Hamster,
  Lock,
  Money,
  MoneyBag,
  Pig1,
  Pig2,
  Temple,
  Wizard,
} from "../index";

export type EmojiName =
  | "ants"
  | "factory-1"
  | "factory-2"
  | "fisher"
  | "hamster"
  | "lock"
  | "money"
  | "moneybag"
  | "pig-1"
  | "pig-2"
  | "temple"
  | "wizard";

interface Props {
  name: EmojiName;
  className?: string;
  detailed?: boolean;
}

const emojis = {
  ant: Ants,
  "factory-1": Factory1,
  "factory-2": Factory2,
  fisher: Fisher,
  hamster: Hamster,
  lock: Lock,
  money: Money,
  moneybag: MoneyBag,
  "pig-1": Pig1,
  "pig-2": Pig2,
  temple: Temple,
  wizard: Wizard,
};

export const Emoji: React.FC<Props> = ({ name, ...props }) => {
  const Image = emojis[name as keyof typeof emojis];

  return <Image {...props} />;
};
