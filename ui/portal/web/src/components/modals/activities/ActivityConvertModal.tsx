import {
  IconArrowDown,
  IconButton,
  IconClose,
  IconLink,
  TruncateText,
  useApp,
} from "@left-curve/applets-kit";
import { forwardRef } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const ActivityConvertModal = forwardRef(() => {
  const { hideModal } = useApp();

  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="p-4 flex flex-col gap-5">
        <h2 className="text-lg font-semibold text-center text-primary-900">
          {m["activities.activity.modal.swapped"]()}
        </h2>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-2 w-full">
              <div className="flex items-center justify-between  h3-bold text-secondary-700">
                <p>10.00 USDC</p>
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                  alt="Token"
                  className="h-8 w-8"
                />
              </div>
              <p className="text-tertiary-500 diatype-sm-regular">$20.00</p>
            </div>
            <div className="border border-gray-300 rounded-full p-1 self-center">
              <IconArrowDown className="h-3 w-3 text-gray-300" />
            </div>
            <div className="flex flex-col gap-2 w-full">
              <div className="flex items-center justify-between  h3-bold text-secondary-700">
                <p>10.00 USDC</p>
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                  alt="Token"
                  className="h-8 w-8"
                />
              </div>
              <p className="text-tertiary-500 diatype-sm-regular">$20.00</p>
            </div>
          </div>
          <span className="w-full h-[1px] bg-secondary-gray my-2" />
          <div className="flex flex-col gap-2 w-full">
            <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
              <p className="diatype-sm-regular text-tertiary-500">
                {m["activities.activity.modal.fee"]()}
              </p>
              <div className="flex items-center gap-1">
                <p>$1.2</p>
              </div>
            </div>
            <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
              <p className="diatype-sm-regular text-tertiary-500">
                {m["activities.activity.modal.time"]()}
              </p>
              <div className="flex items-center gap-1">
                <p>August 14, 2025 10:15 AM</p>
              </div>
            </div>
            <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
              <p className="diatype-sm-regular text-tertiary-500">
                {m["activities.activity.modal.transaction"]()}
              </p>
              <div className="flex items-center gap-1">
                <TruncateText text="0x8dn1...153f" />
                <IconLink className="text-tertiary-500 h-4 w-4" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
});
