import { forwardRef, useEffect } from "react";

import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Button, IconKey, useApp } from "@left-curve/applets-kit";
import { useAccount, useSessionKey } from "@left-curve/store";

export const RenewSession = forwardRef<undefined>(() => {
  const { connector } = useAccount();
  const { hideModal } = useApp();
  const { createSessionKey, session } = useSessionKey();

  useEffect(() => {
    if (session && Date.now() < Number(session.sessionInfo.expireAt)) {
      hideModal();
    }
  }, [session]);

  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative max-w-[400px]">
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-secondary-green flex items-center justify-center text-green-bean-600">
          <IconKey />
        </div>
        <p className="text-secondary-700 h4-bold">{m["modals.renewSession.title"]()}</p>
        <p className="text-tertiary-500 diatype-m-medium">
          {m["modals.renewSession.description"]()}
        </p>
      </div>
      <span className="w-full h-[1px] bg-secondary-gray my-2 lg:block hidden" />
      <div className="p-4 flex gap-4 flex-col-reverse lg:flex-row">
        <Button
          fullWidth
          variant="primary"
          onClick={() => createSessionKey({ expireAt: Date.now() + DEFAULT_SESSION_EXPIRATION })}
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
