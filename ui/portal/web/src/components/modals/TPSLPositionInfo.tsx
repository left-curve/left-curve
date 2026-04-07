import { FormattedNumber } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type TPSLPositionInfoProps = {
  symbol: string;
  isLong: boolean;
  absSize: number;
  entryPrice: string;
  markPrice: string;
};

export const TPSLPositionInfo: React.FC<TPSLPositionInfoProps> = ({
  symbol,
  isLong,
  absSize,
  entryPrice,
  markPrice,
}) => (
  <div className="flex flex-col gap-1">
    <div className="w-full flex gap-2 items-center justify-between">
      <p className="diatype-sm-regular text-ink-tertiary-500">{m["modals.tpsl.coin"]()}</p>
      <p className="diatype-sm-medium text-ink-secondary-700">{symbol}</p>
    </div>
    <div className="w-full flex gap-2 items-center justify-between">
      <p className="diatype-sm-regular text-ink-tertiary-500">{m["modals.tpsl.position"]()}</p>
      <p className={`diatype-sm-medium ${isLong ? "text-utility-success-600" : "text-utility-error-600"}`}>
        {isLong ? m["modals.tpsl.long"]() : m["modals.tpsl.short"]()} {absSize} {symbol}
      </p>
    </div>
    <div className="w-full flex gap-2 items-center justify-between">
      <p className="diatype-sm-regular text-ink-tertiary-500">{m["modals.tpsl.entryPrice"]()}</p>
      <p className="diatype-sm-medium text-ink-secondary-700">
        <FormattedNumber number={entryPrice} formatOptions={{ currency: "USD" }} as="span" />
      </p>
    </div>
    <div className="w-full flex gap-2 items-center justify-between">
      <p className="diatype-sm-regular text-ink-tertiary-500">{m["modals.tpsl.markPrice"]()}</p>
      <p className="diatype-sm-medium text-ink-secondary-700">
        <FormattedNumber number={markPrice} formatOptions={{ currency: "USD" }} as="span" />
      </p>
    </div>
  </div>
);
