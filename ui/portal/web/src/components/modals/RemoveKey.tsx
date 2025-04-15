import { useAccount, useSigningClient } from "@left-curve/store";
import { captureException } from "@sentry/react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import type { KeyHash } from "@left-curve/dango/types";
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
  const queryClient = useQueryClient();
  const { hideModal } = useApp();

  const { mutateAsync: removeKey, isPending } = useMutation({
    mutationFn: async () => {
      if (!account || !signingClient) throw new Error("We couldn't process the request");

      await signingClient.updateKey({
        keyHash,
        sender: account.address,
        action: "delete",
      });
    },
    onError: (e) => captureException(e),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["user_keys"] });
      hideModal();
    },
  });

  return (
    <div className="flex flex-col bg-white-100 rounded-xl relative max-w-[400px]">
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
          <h3 className="h4-bold">{m["settings.keyManagement.management.delete.title"]()}</h3>
          <p className="text-gray-500 diatype-m-regular">
            {m["settings.keyManagement.management.delete.description"]()}
          </p>
          <p className="text-gray-500 diatype-m-regular">
            {m["settings.keyManagement.management.delete.warning"]()}
          </p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2 lg:block hidden" />
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
