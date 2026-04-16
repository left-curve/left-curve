import { forwardRef, useState } from "react";

import { Button, IconButton, IconClose, Input, numberMask, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

type AdjustSlippageProps = {
  currentSlippage: number;
  onConfirm: (slippage: number) => void;
};

export const AdjustSlippage = forwardRef<unknown, AdjustSlippageProps>(
  ({ currentSlippage, onConfirm }, _ref) => {
    const { hideModal } = useApp();
    const [value, setValue] = useState((currentSlippage * 100).toString());

    const handleConfirm = () => {
      const num = Number(value);
      if (num > 0 && num <= 100) {
        onConfirm(num / 100);
        hideModal();
      }
    };

    return (
      <div className="flex flex-col gap-4 bg-surface-primary-rice md:border border-outline-secondary-gray rounded-xl relative px-6 py-6 md:w-[30rem]">
        <IconButton className="absolute right-4 top-4" variant="link" onClick={hideModal}>
          <IconClose />
        </IconButton>

        <div className="flex flex-col gap-2">
          <h2 className="diatype-lg-bold text-ink-primary-900">
            {m["dex.protrade.perps.adjustMaxSlippage"]()}
          </h2>
          <p className="text-ink-tertiary-500 diatype-sm-regular">
            {m["dex.protrade.perps.adjustMaxSlippageDescription"]()}
          </p>
        </div>

        <div className="-mx-6 h-px bg-outline-secondary-gray" />

        <div className="flex flex-col gap-2">
          <p className="exposure-sm-italic text-ink-secondary-700">
            {m["dex.protrade.perps.maxSlippage"]()}
          </p>
          <Input
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="0"
            endContent={<span className="text-ink-tertiary-500 diatype-m-medium pr-3">%</span>}
            classNames={{
              inputWrapper: "h-auto py-3",
              input: "!diatype-m-medium",
            }}
          />
        </div>

        <Button fullWidth onClick={handleConfirm} isDisabled={!value || Number(value) <= 0}>
          {m["common.confirm"]()}
        </Button>
      </div>
    );
  },
);
