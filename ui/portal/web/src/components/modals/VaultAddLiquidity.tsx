import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { forwardRef, useImperativeHandle } from "react";

import { formatNumber } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type VaultAddLiquidityProps = {
  confirmAddLiquidity: () => void;
  rejectAddLiquidity?: () => void;
  amount: string;
  sharesToReceive: string;
};

export const VaultAddLiquidity = forwardRef(
  ({ confirmAddLiquidity, rejectAddLiquidity, amount, sharesToReceive }: VaultAddLiquidityProps, ref) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {
        if (rejectAddLiquidity) rejectAddLiquidity();
      },
    }));

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["vaultLiquidity.modal.addLiquidity"]()}
        </p>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 diatype-sm-regular">
              {m["vaultLiquidity.modal.depositing"]()}
            </p>
            <div className="flex items-center justify-between">
              <p className="text-ink-secondary-700 h3-bold">
                {formatNumber(amount, { ...formatNumberOptions, currency: "USD" })}
              </p>
              <img src="/images/coins/usd.svg" alt="USD" className="w-8 h-8" />
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 diatype-sm-regular">
              {m["vaultLiquidity.modal.toReceive"]()}
            </p>
            <div className="flex items-center justify-between">
              <p className="text-ink-secondary-700 h3-bold">
                {formatNumber(sharesToReceive, formatNumberOptions)} {m["vaultLiquidity.vaultShares"]()}
              </p>
            </div>
          </div>
          <div className="flex items-center justify-between pt-2 border-t border-outline-secondary-gray">
            <p className="text-ink-tertiary-500 diatype-sm-regular">
              {m["vaultLiquidity.modal.destination"]()}
            </p>
            <p className="text-ink-secondary-700 diatype-sm-medium">
              {m["vaultLiquidity.title"]()}
            </p>
          </div>
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => {
            if (rejectAddLiquidity) rejectAddLiquidity();
            hideModal();
          }}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth onClick={() => [confirmAddLiquidity(), hideModal()]}>
          {m["common.confirm"]()}
        </Button>
      </div>
    );
  },
);
