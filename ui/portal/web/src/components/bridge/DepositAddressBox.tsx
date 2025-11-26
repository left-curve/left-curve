import { TextCopy } from "@left-curve/applets-kit";
import React from "react";

export const DepositAddressBox: React.FC = () => {
  return (
    <div className="flex flex-col gap-1 w-full">
      <p className="exposure-sm-italic text-ink-secondary-700">Deposit Address</p>
      <div className="flex flex-col md:flex-row gap-4 p-4 rounded-xl shadow-account-card bg-surface-secondary-rice">
        <div className="border border-ink-primary-900 flex items-center justify-center min-w-[100px] min-h-[100px]">
          QR
        </div>
        <div className="flex-1 flex flex-col gap-3">
          <p className="diatype-sm-regular text-ink-tertiary-500 h-[36px]">
            This address can only receive assets on{" "}
            <span className="text-ink-secondary-700 diatype-sm-bold">Bitcoin network</span>
          </p>
          <span className="h-[1px] w-full bg-outline-secondary-gray" />
          <div className="flex items-start gap-1 h-[44px]">
            <p className="text-ink-secondary-700 break-all text-start diatype-m-medium">
              0x7f87b9f3dcc4c5a9c17fa00323508e580a123456
            </p>
            <TextCopy copyText="0x7f87b9f3dcc4c5a9c17fa00323508e580a123456" />
          </div>
        </div>
      </div>
    </div>
  );
};
