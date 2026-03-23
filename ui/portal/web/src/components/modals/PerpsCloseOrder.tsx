import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { forwardRef } from "react";

export const PerpsCloseOrder = forwardRef<void, { orderId: string; isConditional?: boolean }>(
  ({ orderId, isConditional = false }) => {
    const { hideModal } = useApp();
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const { isPending, mutateAsync: cancelOrder } = useSubmitTx({
      submission: {
        success: m["dex.protrade.allOrdersCancelled"](),
        error: m["errors.failureRequest"](),
      },
      mutation: {
        mutationFn: async () => {
          if (!signingClient) throw new Error("No signing client available");
          if (isConditional) {
            await signingClient.cancelConditionalOrder({
              sender: account!.address,
              request: { one: orderId },
            });
          } else {
            await signingClient.cancelPerpsOrder({
              sender: account!.address,
              request: { one: orderId },
            });
          }
        },
        onSuccess: () => {
          hideModal();
        },
      },
    });

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <h2 className="text-ink-primary-900 h4-bold w-full">
          {m["modals.proTradeCloseOrder.title"]()}
        </h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["modals.proTradeCloseOrder.description"]()}
        </p>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth isLoading={isPending} onClick={() => cancelOrder()}>
          {m["modals.proTradeCloseOrder.action"]()}
        </Button>
      </div>
    );
  },
);
