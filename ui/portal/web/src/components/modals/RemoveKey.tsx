import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { forwardRef } from "react";

import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { Button, IconButton, IconClose, IconTrash } from "@left-curve/applets-kit";

import type { KeyHash } from "@left-curve/dango/types";
interface Props {
  keyHash: KeyHash;
}

export const RemoveKey = forwardRef<never, Props>(({ keyHash }, _ref) => {
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { hideModal } = useApp();

  const { mutateAsync: removeKey, isPending } = useSubmitTx({
    mutation: {
      invalidateKeys: [["user_keys"]],
      mutationFn: async () => {
        if (!account || !signingClient) throw new Error("We couldn't process the request");

        await signingClient.updateKey({
          keyHash,
          sender: account.address,
          action: "delete",
        });
      },
      onSuccess: () => hideModal(),
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative max-w-[400px]">
      <IconButton
        className="hidden lg:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-red-bean-100 flex items-center justify-center text-red-bean-600">
          <IconTrash />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="h4-bold text-primary-900">
            {m["settings.keyManagement.management.delete.title"]()}
          </h3>
          <p className="text-tertiary-500 diatype-m-regular">
            {m["settings.keyManagement.management.delete.description"]()}
          </p>
          <p className="text-tertiary-500 diatype-m-regular">
            {m["settings.keyManagement.management.delete.warning"]()}
          </p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-secondary-gray my-2 lg:block hidden" />
      <div className="p-4 flex gap-4 flex-col-reverse lg:flex-row">
        <Button fullWidth variant="secondary" onClick={() => hideModal()} isDisabled={isPending}>
          {m["common.cancel"]()}
        </Button>
        <Button fullWidth onClick={() => removeKey()} isLoading={isPending}>
          {m["common.delete"]()}
        </Button>
      </div>
    </div>
  );
});
