import {
  Button,
  IconButton,
  IconClose,
  IconTrash,
  useSigningClient,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import type { KeyHash } from "@left-curve/dango/types";
import { wait } from "@left-curve/dango/utils";
import type React from "react";
import { useApp } from "~/hooks/useApp";

interface Props {
  keyHash: KeyHash;
}

export const RemoveKey: React.FC<Props> = ({ keyHash }) => {
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
      await wait(1500);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["user_keys"] });
      hideModal();
    },
  });

  return (
    <div className="flex flex-col bg-white-100 rounded-3xl relative max-w-[400px]">
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
          <h3 className="h4-bold">Delete passkey</h3>
          <p className="text-gray-500 diatype-m-regular">
            Are you sure you want to delete this passkey?
          </p>
          <p className="text-gray-500 diatype-m-regular">This action cannot be undone.</p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2 lg:block hidden" />
      <div className="p-4 flex gap-4 flex-col-reverse lg:flex-row">
        <Button fullWidth variant="secondary" onClick={() => hideModal()} isDisabled={isPending}>
          Cancel
        </Button>
        <Button fullWidth onClick={() => removeKey()} isLoading={isPending}>
          Delete
        </Button>
      </div>
    </div>
  );
};
