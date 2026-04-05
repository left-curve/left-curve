import { Button, IconButton, IconClose, WarningContainer, useApp } from "@left-curve/applets-kit";

import { forwardRef, useImperativeHandle } from "react";

import { formatNumber } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type VaultWithdrawLiquidityProps = {
  confirmWithdrawal: () => void;
  rejectWithdrawal?: () => void;
  sharesToBurn: string;
  usdToReceive: string;
};

export const VaultWithdrawLiquidity = forwardRef(
  ({ confirmWithdrawal, rejectWithdrawal, usdToReceive }: VaultWithdrawLiquidityProps, ref) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {
        if (rejectWithdrawal) rejectWithdrawal();
      },
    }));

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["vaultLiquidity.modal.withdrawLiquidity"]()}
        </p>

        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => {
            if (rejectWithdrawal) rejectWithdrawal();
            hideModal();
          }}
        >
          <IconClose />
        </IconButton>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 exposure-sm-italic">
              {m["vaultLiquidity.modal.withdrawing"]()}
            </p>
            <div className="flex items-center justify-between">
              <p className="text-ink-secondary-700 h3-bold">
                {formatNumber(usdToReceive, { ...formatNumberOptions, currency: "USD" })}
              </p>
              <img src="/images/coins/usd.svg" alt="USD" className="w-8 h-8" />
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 exposure-sm-italic">
              {m["vaultLiquidity.modal.from"]()}
            </p>
            <p className="text-ink-secondary-700 diatype-m-bold">
              {m["vaultLiquidity.title"]()}
            </p>
          </div>

          <div className="flex items-center justify-between">
            <p className="text-ink-tertiary-500 diatype-sm-regular">
              {m["vaultLiquidity.networkFee"]()}
            </p>
            <p className="text-ink-secondary-700 diatype-sm-medium">
              {formatNumber("0.02", { ...formatNumberOptions, currency: "USD" })}
            </p>
          </div>

          <WarningContainer
            title={m["vaultLiquidity.modal.cooldownTitle"]()}
            description={m["vaultLiquidity.modal.cooldownDescription"]()}
          />
        </div>

        <Button fullWidth onClick={() => [confirmWithdrawal(), hideModal()]}>
          {m["common.confirm"]()}
        </Button>
      </div>
    );
  },
);
