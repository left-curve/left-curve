import { useEffect } from "react";

import { Button, IconButton, IconClose, Input, numberMask, useApp } from "@left-curve/applets-kit";
import { Decimal } from "@left-curve/dango/utils";
import { useInputs } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useStorage } from "@left-curve/store";
import { PERPS_DEFAULT_SLIPPAGE } from "~/constants";

const MAX_SLIPPAGE_PERCENT = 5;
const MAX_DECIMAL_PLACES = 2;

const validateSlippage = (value: string): boolean | string => {
  if (!value) return true;

  const decimals = value.includes(".") ? value.split(".")[1]?.length ?? 0 : 0;
  if (decimals > MAX_DECIMAL_PLACES) {
    return m["dex.protrade.perps.slippageMaxDecimals"]({ max: MAX_DECIMAL_PLACES.toString() });
  }

  const percent = Decimal(value);
  if (percent.lte(0) || percent.gt(MAX_SLIPPAGE_PERCENT)) {
    return m["dex.protrade.perps.slippageOutOfRange"]({ max: MAX_SLIPPAGE_PERCENT.toString() });
  }

  return true;
};

export function AdjustSlippage() {
  const { hideModal } = useApp();
  const [maxSlippage, setMaxSlippage] = useStorage<string>("perps-max-slippage", {
    initialValue: PERPS_DEFAULT_SLIPPAGE,
  });

  const { register, inputs, setValue, isValid } = useInputs({
    initialValues: {
      slippage: Decimal(maxSlippage).mul(100).toString(),
    },
  });

  useEffect(() => {
    setValue("slippage", Decimal(maxSlippage).mul(100).toString());
  }, [maxSlippage, setValue]);

  const handleConfirm = () => {
    if (!isValid) return;
    const percent = Decimal(inputs.slippage?.value || 0);
    setMaxSlippage(percent.div(100).toString());
    hideModal();
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
          {...register("slippage", {
            mask: numberMask,
            validate: validateSlippage,
            strategy: "onChange",
          })}
          placeholder="0"
          endContent={<span className="text-ink-tertiary-500 diatype-m-medium pr-3">%</span>}
          classNames={{
            inputWrapper: "h-auto py-3",
            input: "!diatype-m-medium",
          }}
        />
      </div>

      <Button fullWidth onClick={handleConfirm} isDisabled={!inputs.slippage?.value || !isValid}>
        {m["common.confirm"]()}
      </Button>
    </div>
  );
}
