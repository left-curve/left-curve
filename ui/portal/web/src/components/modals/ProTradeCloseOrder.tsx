import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef } from "react";

import type { OrderId } from "@left-curve/dango/types";

export const ProTradeCloseOrder = forwardRef<void, { orderId: OrderId }>(({ orderId }) => {
  const { hideModal } = useApp();
  const { account } = useAccount();
  const queryClient = useQueryClient();
  const { data: signingClient } = useSigningClient();
  const { isPending, mutateAsync: cancelOrder } = useSubmitTx({
    submission: {
      success: m["dex.protrade.allOrdersCancelled"](),
      error: m["errors.failureRequest"](),
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        await signingClient.batchUpdateOrders({
          cancels: { some: [orderId] },
          sender: account!.address,
        });
      },
      onSuccess: async () => {
        await queryClient.invalidateQueries({
          queryKey: ["ordersByUser", account?.address],
        });
        hideModal();
      },
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
      <h2 className="text-primary-900 h4-bold w-full">{m["modals.proTradeCloseOrder.title"]()}</h2>
      <p className="text-tertiary-500 diatype-sm-regular">
        {m["modals.proTradeCloseOrder.description"]()}
      </p>
      {/* <RadioGroup name="close-positions-all" defaultValue="market-close">
        <Radio value="market-close" label="Market Close" />
        <Radio value="limit-close" label="Limit Close at Mid Price" />
      </RadioGroup> */}
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
});
