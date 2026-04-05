import { Range, type RangeProps } from "./Range";
import { Button } from "./Button";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { twMerge } from "tailwind-merge";

import type React from "react";

type RangeWithButtonsProps = {
  amount: string;
  balance: string;
  setValue: (value: string) => void;
  setActiveInput: () => void;
  className?: string;
  classNames?: RangeProps["classNames"];
};

export const RangeWithButtons: React.FC<RangeWithButtonsProps> = (parameters) => {
  const { amount, balance, setValue, setActiveInput, className, classNames } = parameters;

  return (
    <div className={twMerge("flex flex-col gap-4", className)}>
      <Range
        minValue={0}
        maxValue={Number(balance)}
        value={Number(amount)}
        onChange={(value) => {
          setActiveInput();
          setValue(String(value));
        }}
        classNames={{
          ...classNames,
          inputWrapper: twMerge("px-0", classNames?.inputWrapper),
        }}
        showPercentage
      />
      <div className="w-full flex gap-4 justify-end">
        <Button
          type="button"
          variant="tertiary-red"
          size="xs"
          className="py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(String(Number(balance) * 0.25));
          }}
        >
          25%
        </Button>{" "}
        <Button
          type="button"
          variant="tertiary-red"
          size="xs"
          className="py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(String(Number(balance) * 0.5));
          }}
        >
          50%
        </Button>{" "}
        <Button
          type="button"
          variant="tertiary-red"
          size="xs"
          className="py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(String(Number(balance) * 0.75));
          }}
        >
          75%
        </Button>{" "}
        <Button
          type="button"
          variant="tertiary-red"
          size="xs"
          className="py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(balance);
          }}
        >
          {m["common.max"]()}
        </Button>
      </div>
    </div>
  );
};
