import { Button, IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { forwardRef } from "react";

export const PerpsCloseAll = forwardRef<void, Record<string, never>>(() => {
  const { hideModal } = useApp();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { isPending, mutateAsync: cancelAllOrders } = useSubmitTx({
    submission: {
      success: m["dex.protrade.allOrdersCancelled"](),
      error: m["errors.failureRequest"](),
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("No signing client available");
        await signingClient.cancelPerpsOrder({
          sender: account!.address,
          request: "all",
        });
      },
      onSuccess: () => {
        hideModal();
      },
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
      <h2 className="text-ink-primary-900 h4-bold w-full">
        {m["modals.protradeCloseAllOrders.title"]()}
      </h2>
      <p className="text-ink-tertiary-500 diatype-sm-regular">
        {m["modals.protradeCloseAllOrders.description"]()}
      </p>
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
