import { Button, IconButton, IconClose, IconWarningTriangle, useApp } from "@left-curve/applets-kit";

import { forwardRef, useImperativeHandle } from "react";

import { formatNumber } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type VaultWithdrawLiquidityWithPenaltyProps = {
  confirmWithdrawal: () => void;
  rejectWithdrawal?: () => void;
  usdToWithdraw: string;
  penaltyPercentage: number;
  penaltyEndDate: string;
};

export const VaultWithdrawLiquidityWithPenalty = forwardRef(
  (
    {
      confirmWithdrawal,
      rejectWithdrawal,
      usdToWithdraw,
      penaltyPercentage,
      penaltyEndDate,
    }: VaultWithdrawLiquidityWithPenaltyProps,
    ref,
  ) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;

    const penaltyAmount = (Number(usdToWithdraw) * penaltyPercentage) / 100;
    const amountAfterPenalty = Number(usdToWithdraw) - penaltyAmount;

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {
        if (rejectWithdrawal) rejectWithdrawal();
      },
    }));

    const handleCancel = () => {
      if (rejectWithdrawal) rejectWithdrawal();
      hideModal();
    };

    const handleConfirm = () => {
      confirmWithdrawal();
      hideModal();
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <div className="flex items-start justify-between">
          <div className="w-10 h-10 rounded-full bg-primitives-red-light-100 flex items-center justify-center">
            <IconWarningTriangle className="w-5 h-5 text-primitives-red-light-600" />
          </div>
          <IconButton
            className="hidden md:block"
            variant="link"
            onClick={handleCancel}
          >
            <IconClose />
          </IconButton>
        </div>

        <div className="flex flex-col gap-1">
          <p className="text-ink-primary-900 diatype-lg-medium">
            {m["vaultLiquidity.modal.earlyWithdrawalTitle"]()}
          </p>
          <p className="text-ink-tertiary-500 diatype-sm-regular">
            {m["vaultLiquidity.modal.earlyWithdrawalDescription"]({ date: penaltyEndDate })}
          </p>
        </div>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 exposure-sm-italic">
              {m["vaultLiquidity.modal.withdrawing"]()}
            </p>
            <div className="flex items-center justify-between">
              <p className="text-ink-secondary-700 h3-bold">
                {formatNumber(usdToWithdraw, { ...formatNumberOptions, currency: "USD" })}
              </p>
              <img src="/images/coins/usd.svg" alt="USD" className="w-8 h-8" />
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 exposure-sm-italic">
              {m["vaultLiquidity.modal.penalty"]()}{" "}
              <span className="text-primitives-red-light-600">({penaltyPercentage}%)</span>
            </p>
            <p className="text-primitives-red-light-600 h3-bold">
              -{formatNumber(String(penaltyAmount), { ...formatNumberOptions, currency: "USD" })}
            </p>
          </div>

          <div className="flex flex-col gap-1">
            <p className="text-ink-tertiary-500 exposure-sm-italic">
              {m["vaultLiquidity.modal.youReceive"]()}
            </p>
            <p className="text-ink-secondary-700 h3-bold">
              {formatNumber(String(amountAfterPenalty), { ...formatNumberOptions, currency: "USD" })}
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
        </div>

        <div className="flex gap-3">
          <Button fullWidth variant="secondary" onClick={handleCancel}>
            {m["common.cancel"]()}
          </Button>
          <Button fullWidth onClick={handleConfirm}>
            {m["common.withdraw"]()}
          </Button>
        </div>
      </div>
    );
  },
);
