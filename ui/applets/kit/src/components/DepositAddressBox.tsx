import { TextCopy } from "./TextCopy";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type DepositAddressBoxProps = {
  network: string;
  address: string;
};

export const DepositAddressBox: React.FC<DepositAddressBoxProps> = ({ network, address }) => {
  return (
    <div className="flex flex-col gap-1 w-full">
      <p className="exposure-sm-italic text-ink-secondary-700">{m["bridge.depositAddress"]()}</p>
      <div className="flex flex-col md:flex-row gap-4 p-4 rounded-xl shadow-account-card bg-surface-secondary-rice">
        <div className="border border-ink-primary-900 flex items-center justify-center min-w-[100px] min-h-[100px]">
          {m["common.qr"]()}
        </div>
        <div className="flex-1 flex flex-col gap-3">
          <p className="diatype-sm-regular text-ink-tertiary-500 h-[36px]">
            {m["bridge.onlyCanReceiveOn"]()}{" "}
            <span className="text-ink-secondary-700 diatype-sm-bold">
              {m["bridge.networkName"]({ network })}
            </span>
          </p>
          <span className="h-[1px] w-full bg-outline-secondary-gray" />
          <div className="flex items-start gap-1 h-[44px]">
            <p className="text-ink-secondary-700 break-all text-start diatype-m-medium">
              {address}
            </p>
            <TextCopy copyText={address} />
          </div>
        </div>
      </div>
    </div>
  );
};
