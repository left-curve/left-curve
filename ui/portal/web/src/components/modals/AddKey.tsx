import { useAccount, useConnectors, useSigningClient, useSubmitTx } from "@left-curve/store";
import { forwardRef } from "react";

import { IconButton, IconClose, IconKey, useApp } from "@left-curve/applets-kit";
import { AuthOptions } from "../auth/AuthOptions";

import { m } from "~/paraglide/messages";

export const AddKeyModal = forwardRef((_props, _ref) => {
  const connectors = useConnectors();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { hideModal } = useApp();

  const { mutateAsync: addKey, isPending } = useSubmitTx({
    mutation: {
      invalidateKeys: [["user_keys"], ["quests", account?.username]],
      mutationFn: async (connectorId: string) => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("Connector not found");
        if (!account || !signingClient) throw new Error("We couldn't process the request");

        const { keyHash, key } = await connector.createNewKey!();

        await signingClient?.updateKey({
          keyHash,
          sender: account.address,
          action: {
            insert: key,
          },
        });
      },
      onSuccess: () => hideModal(),
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-secondary-green flex items-center justify-center text-green-bean-600">
          <IconKey />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="h4-bold text-primary-900">
            {m["settings.keyManagement.management.add.title"]()}
          </h3>
          <p className="text-tertiary-500 diatype-m-regular">
            {m["settings.keyManagement.management.add.description"]()}
          </p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-secondary-gray my-2" />
      <div className="p-4">
        <AuthOptions mode="signin" action={addKey} isPending={isPending} />
      </div>
    </div>
  );
});
