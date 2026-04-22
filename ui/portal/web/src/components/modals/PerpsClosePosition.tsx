import { Button, FormattedNumber, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { PERPS_DEFAULT_SLIPPAGE } from "~/constants";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef } from "react";

type PerpsClosePositionProps = {
  pairId: string;
  size: string;
  pnl: number;
};

export const PerpsClosePosition = forwardRef<void, PerpsClosePositionProps>(
  ({ pairId, size, pnl }) => {
    const { hideModal } = useApp();
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const queryClient = useQueryClient();

    const sizeNum = Number(size);
    const closeSize = sizeNum > 0 ? `-${Math.abs(sizeNum)}` : `${Math.abs(sizeNum)}`;
    const isProfit = pnl >= 0;

    const { isPending, mutateAsync: closePosition } = useSubmitTx({
      submission: {
        success: "Position closed successfully",
      },
      mutation: {
        mutationFn: async () => {
          if (!signingClient) throw new Error("No signing client available");
          await signingClient.submitPerpsOrder({
            sender: account!.address,
            pairId,
            size: closeSize,
            kind: { market: { maxSlippage: PERPS_DEFAULT_SLIPPAGE } },
            reduceOnly: true,
          });
        },
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: ["prices"] });
          queryClient.invalidateQueries({ queryKey: ["perpsTradeHistory", account?.address] });
          hideModal();
        },
      },
    });

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <h2 className="text-ink-primary-900 h4-bold w-full">Close Position</h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          This will market-close your {pairId.replace("perp/", "").toUpperCase()} position.
        </p>
        <div className="flex flex-col gap-1">
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">Size</p>
            <p className="diatype-sm-medium text-ink-secondary-700">
              {Math.abs(sizeNum).toString()}
            </p>
          </div>
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">PNL</p>
            <p
              className={`diatype-sm-medium ${isProfit ? "text-utility-success-600" : "text-utility-error-600"}`}
            >
              {isProfit ? "+" : ""}
              <FormattedNumber
                number={pnl.toString()}
                formatOptions={{ currency: "USD" }}
                as="span"
              />
            </p>
          </div>
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth isLoading={isPending} onClick={() => closePosition()}>
          Confirm Close
        </Button>
      </div>
    );
  },
);
