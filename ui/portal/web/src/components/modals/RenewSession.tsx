import { forwardRef } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";

import { Button, IconButton, IconClose, IconKey } from "@left-curve/applets-kit";
import { useAccount, useSessionKey } from "@left-curve/store";

const TWENTY_FOUR_HOURS = 24 * 60 * 60 * 1000;

export const RenewSession = forwardRef<undefined>(() => {
  const { connector } = useAccount();
  const { hideModal } = useApp();
  const { createSessionKey } = useSessionKey();

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
        <p className="text-gray-700 h4-bold">{m["modals.renewSession.title"]()}</p>
        <p className="text-gray-500 diatype-m-medium">{m["modals.renewSession.description"]()}</p>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2 lg:block hidden" />
      <div className="p-4 flex gap-4 flex-col-reverse lg:flex-row">
        <Button
          fullWidth
          variant="primary"
          onClick={() => [
            createSessionKey({ expireAt: Date.now() + TWENTY_FOUR_HOURS }),
            hideModal(),
          ]}
        >
          {m["common.signin"]()}
        </Button>
        <Button
          fullWidth
          variant="secondary"
          onClick={() => [connector?.disconnect(), hideModal()]}
        >
          {m["modals.renewSession.stayLogout"]()}
        </Button>
      </div>
    </div>
  );
});
