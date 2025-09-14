import { Button, Range } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type RangeWithButtonsProps = {
  amount: string;
  balance: string;
  setValue: (value: string) => void;
  setActiveInput: () => void;
};

export const RangeWithButtons: React.FC<RangeWithButtonsProps> = (parameters) => {
  const { amount, balance, setValue, setActiveInput } = parameters;
  return (
    <div className="flex flex-col gap-4">
      <Range
        minValue={0}
        maxValue={Number(balance)}
        step={0.1}
        value={Number(amount)}
        onChange={(value) => {
          setActiveInput();
          setValue(String(value));
        }}
        classNames={{ inputWrapper: "px-0" }}
        showPercentage
      />
      <div className="w-full flex gap-4 justify-end">
        <Button
          type="button"
          variant="secondary"
          size="xs"
          className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(String(Number(balance) * 0.25));
          }}
        >
          25%
        </Button>{" "}
        <Button
          type="button"
          variant="secondary"
          size="xs"
          className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
          onClick={() => {
            setActiveInput();
            setValue(String(Number(balance) * 0.5));
          }}
        >
          50%
        </Button>{" "}
        <Button
          type="button"
          variant="secondary"
          size="xs"
          className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
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
