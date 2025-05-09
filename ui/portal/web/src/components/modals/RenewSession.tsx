import { forwardRef } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { Button, IconButton, IconClose, IconKey, IconTrash } from "@left-curve/applets-kit";

export const RenewSession = forwardRef<undefined>(() => {
  const { hideModal } = useApp();

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
        <div className="w-12 h-12 rounded-full bg-green-bean-100 flex items-center justify-center text-green-bean-600">
          <IconKey />
        </div>
        <p className="text-gray-700 h4-bold">Renew Your Session Key</p>
        <p className="text-gray-500 diatype-m-medium">
          Your session key is not active or has expired. Would you like to start a new session to
          use the app?
        </p>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2 lg:block hidden" />
      <div className="p-4 flex gap-4 flex-col-reverse lg:flex-row">
        <Button fullWidth variant="primary" onClick={() => hideModal()}>
          Activate session key
        </Button>
        <Button fullWidth variant="secondary" onClick={() => hideModal()}>
          Do it later
        </Button>
      </div>
    </div>
  );
});
