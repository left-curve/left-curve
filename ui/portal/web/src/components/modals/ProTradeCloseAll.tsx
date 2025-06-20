import { Button, IconButton, IconClose } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { useAccount, useSigningClient } from "@left-curve/store";
import { useMutation } from "@tanstack/react-query";
import { forwardRef } from "react";

export const ProTradeCloseAll = forwardRef(() => {
  const { hideModal, subscriptions } = useApp();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();

  const { isPending, mutateAsync: cancelAllOrders } = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("No signing client available");
      subscriptions.emit("submitTx", { isSubmitting: true });
      await signingClient
        .batchUpdateOrders({ cancels: "all", sender: account!.address })
        .then(() => {
          subscriptions.emit("submitTx", {
            isSubmitting: false,
            txResult: { hasSucceeded: true, message: m["dex.protrade.allOrdersCancelled"]() },
          });
        })
        .catch(() => {
          subscriptions.emit("submitTx", {
            isSubmitting: false,
            txResult: { hasSucceeded: false, message: m["errors.failureRequest"]() },
          });
        });
    },
    onSuccess: () => hideModal(),
  });

  return (
    <div className="flex flex-col bg-white-100 md:border border-gray-100 pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
      <h2 className="text-gray-900 h4-bold w-full">{m["modals.protradeCloseAllOrders.title"]()}</h2>
      <p className="text-gray-500 diatype-sm-regular">
        {m["modals.protradeCloseAllOrders.description"]()}
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
      <Button fullWidth isLoading={isPending} onClick={() => cancelAllOrders()}>
        {m["modals.protradeCloseAllOrders.action"]()}
      </Button>
    </div>
  );
});
