import { Button, IconAlert, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { forwardRef, useImperativeHandle } from "react";

import { m } from "~/paraglide/messages";

type PoolWithdrawLiquidityProps = {
  confirmWithdrawal: () => void;
  rejectWithdrawal?: () => void;
};

export const PoolWithdrawLiquidity = forwardRef(
  ({ confirmWithdrawal, rejectWithdrawal }: PoolWithdrawLiquidityProps, ref) => {
    const { hideModal } = useApp();

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {
        if (rejectWithdrawal) rejectWithdrawal();
      },
    }));

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <div className="p-4 flex flex-col gap-4">
          <div className="w-12 h-12 rounded-full bg-red-bean-100 flex items-center justify-center text-red-bean-600">
            <IconAlert />
          </div>
          <div className="flex flex-col gap-2">
            <h3 className="h4-bold text-primary-900">
              {m["poolLiquidity.modal.withdrawalConfirmation"]()}
            </h3>
            <p className="text-tertiary-500 diatype-m-regular">
              {m["poolLiquidity.modal.withdrawPenaltyAdvice"]()}
            </p>
          </div>
        </div>
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
        <Button fullWidth onClick={() => [confirmWithdrawal(), hideModal()]}>
          {m["poolLiquidity.modal.continueWithdraw"]()}
        </Button>
      </div>
    );
  },
);
