import { IconButton, IconClose, IconKey, forwardRef } from "@left-curve/applets-kit";
import { wait } from "@left-curve/dango/utils";
import { useAccount, useConnectors, useSigningClient } from "@left-curve/store";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useApp } from "~/hooks/useApp";
import { AuthOptions } from "../auth/AuthOptions";

import { m } from "~/paraglide/messages";

export const AddKeyModal = forwardRef((_props, _ref) => {
  const connectors = useConnectors();
  const { account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const queryClient = useQueryClient();
  const { hideModal } = useApp();

  const { mutateAsync: addKey, isPending } = useMutation({
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
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["user_keys"] });
      queryClient.invalidateQueries({ queryKey: ["quests", account] });
      hideModal();
    },
  });

  return (
    <div className="flex flex-col bg-white-100 rounded-xl relative">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-green-bean-100 flex items-center justify-center text-green-bean-600">
          <IconKey />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="h4-bold">{m["settings.keyManagment.managment.add.title"]()}</h3>
          <p className="text-gray-500 diatype-m-regular">
            {m["settings.keyManagment.managment.add.description"]()}
          </p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2" />
      <div className="p-4">
        <AuthOptions mode="signin" action={addKey} isPending={isPending} />
      </div>
    </div>
  );
});
