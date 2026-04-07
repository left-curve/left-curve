import { Button, IconButton, IconClose, Range, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { perpsTradeSettingsStore } from "@left-curve/store";
import { forwardRef, useState } from "react";

type PerpsAdjustLeverageProps = {
  perpsPairId: string;
  baseSymbol: string;
  maxLeverage: number;
};

export const PerpsAdjustLeverage = forwardRef<void, PerpsAdjustLeverageProps>(
  ({ perpsPairId, baseSymbol, maxLeverage }) => {
    const { hideModal } = useApp();
    const storedLeverage = perpsTradeSettingsStore((s) => s.leverageByPair[perpsPairId]);
    const setLeverage = perpsTradeSettingsStore((s) => s.setLeverage);

    const initial = Math.min(Math.max(storedLeverage ?? maxLeverage, 1), maxLeverage);
    const [value, setValue] = useState<number>(initial);

    const onConfirm = () => {
      setLeverage(perpsPairId, value, maxLeverage);
      hideModal();
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[28rem]">
        <div className="flex flex-col gap-2 text-center">
          <h2 className="text-ink-primary-900 h4-bold w-full">
            {m["modals.adjustLeverage.title"]()}
          </h2>
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["modals.adjustLeverage.description"]({
              symbol: baseSymbol,
              maxLeverage: String(maxLeverage),
            })}
          </p>
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["modals.adjustLeverage.subDescription"]()}
          </p>
        </div>

        <Range
          minValue={1}
          maxValue={Math.max(maxLeverage, 1)}
          step={1}
          value={value}
          onChange={(v) => setValue(Math.round(v))}
          showSteps
          withInput
          inputEndContent="x"
          classNames={{ input: "max-w-[5rem]" }}
        />

        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>

        <Button fullWidth onClick={onConfirm}>
          {m["modals.adjustLeverage.confirm"]()}
        </Button>

        <div className="border border-utility-error-300 rounded-lg p-3">
          <p className="diatype-xs-regular text-utility-error-600 text-center">
            {m["modals.adjustLeverage.warning"]()}
          </p>
        </div>
      </div>
    );
  },
);
