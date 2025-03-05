import type React from "react";
import { Ants } from "./icons/emoji/Ants";
import { Factory1 } from "./icons/emoji/Factory1";
import { Factory2 } from "./icons/emoji/Factory2";
import { Fisher } from "./icons/emoji/Fisher";
import { Hamster } from "./icons/emoji/Hamster";
import { Lock } from "./icons/emoji/Lock";
import { MapExplorer } from "./icons/emoji/Map";
import { Money } from "./icons/emoji/Money";
import { MoneyBag } from "./icons/emoji/MoneyBag";
import { Pig1 } from "./icons/emoji/Pig1";
import { Pig2 } from "./icons/emoji/Pig2";
import { Temple } from "./icons/emoji/Temple";
import { Wizard } from "./icons/emoji/Wizard";

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
  | "wizard"
  | "map";

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
  map: MapExplorer,
};

export const Emoji: React.FC<Props> = ({ name, ...props }) => {
  const Image = emojis[name as keyof typeof emojis];

  return <Image {...props} />;
};
