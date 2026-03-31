import { FormattedNumber } from "./FormattedNumber";
import { Button } from "./Button";
import { Input } from "./Input";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { numberMask, type useInputs } from "@left-curve/foundation";

export const InputSizeWithMax: React.FC<{
  isDisabled: boolean;
  maxSizeAmount: number;
  availableAmount: string;
  register: ReturnType<typeof useInputs>["register"];
  setValue: ReturnType<typeof useInputs>["setValue"];
  validationMessage?: string;
  startContent?: React.ReactNode;
  endContent?: string;
  label?: string;
  minSizeAmount?: number;
  minSizeMessage?: string;
  hideMaxControls?: boolean;
}> = ({
  isDisabled,
  maxSizeAmount,
  availableAmount,
  register,
  setValue,
  validationMessage,
  startContent,
  endContent,
  label = "Size",
  minSizeAmount,
  minSizeMessage,
  hideMaxControls,
}) => (
  <Input
    placeholder="0"
    isDisabled={isDisabled}
    label={label}
    {...register("size", {
      strategy: "onChange",
      mask: numberMask,
      validate: (v) => {
        const num = Number(v);
        if (num > 0 && minSizeAmount && num < minSizeAmount)
          return minSizeMessage ?? `Min size: ${minSizeAmount}`;
        if (num > Number(maxSizeAmount)) return validationMessage ?? "Exceeds available";
        return true;
      },
    })}
    classNames={{
      base: "z-20",
      inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
      inputParent: "h-[34px] diatype-lg-medium min-w-0",
      input: "!diatype-lg-medium",
    }}
    startText="right"
    startContent={startContent}
    endContent={endContent}
    insideBottomComponent={
      hideMaxControls ? undefined : (
        <div className="flex items-center justify-between gap-2 w-full h-[22px] text-ink-tertiary-500 diatype-sm-regular pl-4">
          <div className="flex items-center gap-2">
            <FormattedNumber number={availableAmount} />
            <Button
              type="button"
              variant="tertiary-red"
              size="xs"
              className="py-[2px] px-[6px] cursor-pointer"
              onClick={() => setValue("size", maxSizeAmount.toString())}
            >
              {m["common.max"]()}
            </Button>
          </div>
        </div>
      )
    }
  />
);
