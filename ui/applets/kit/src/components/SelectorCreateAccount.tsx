import { IconCheckedCircle } from "./icons/IconCheckedCircle";

import { twMerge } from "../utils/twMerge.js";

import type { AccountTypes } from "@left-curve/dango/types";
import type React from "react";

interface Props {
  accountType: AccountTypes;
  onClick?: () => void;
  isSelected?: boolean;
}

const subtext = {
  spot: "Can hold any asset and partake in any activity; cheapest gas cost; can only take over-collateralized loans.",
  margin:
    "Can take under-collateralized loans up to 5x leverage; can only hold whitelisted assets and partake in whitelisted activities.",
  multi: "",
};

export const SelectorCreateAccount: React.FC<Props> = ({ accountType, isSelected, onClick }) => {
  return (
    <div
      className={twMerge(
        "min-h-[9.125rem] w-full max-w-[22.5rem] border border-transparent text-start rounded-md overflow-hidden relative p-4 flex flex-col gap-4 transition-all shadow-account-card items-start justify-start",
        { "cursor-pointer": onClick },
        { " border border-red-bean-400": isSelected },
        {
          "bg-[linear-gradient(98.89deg,_rgba(255,_251,_245,_0.5)_5.88%,_rgba(249,_226,_226,_0.5)_46.73%,_rgba(255,_251,_244,_0.5)_94.73%)]":
            accountType === "spot",
        },
        {
          "bg-[linear-gradient(0deg,_#FFFCF6,_#FFFCF6),linear-gradient(98.89deg,_rgba(248,_249,_239,_0.5)_5.88%,_rgba(239,_240,_195,_0.5)_46.73%,_rgba(248,_249,_239,_0.5)_94.73%)]":
            accountType === "margin",
        },
      )}
      onClick={onClick}
    >
      <p className="capitalize exposure-m-italic">{accountType} Account</p>
      <p className="diatype-sm-medium text-tertiary-500 relative max-w-[15.5rem] z-10 ">
        {subtext[accountType]}
      </p>
      <img
        src={`./images/account-creation/${accountType}.svg`}
        alt={`create-account-${accountType}`}
        className={twMerge("absolute right-0 bottom-0 drag-none select-none", {
          "right-2": accountType === "margin",
        })}
      />
      <IconCheckedCircle
        className={twMerge("w-5 h-5 absolute right-3 top-3 opacity-0 transition-all text-red-400", {
          "opacity-1": isSelected,
        })}
      />
    </div>
  );
};
