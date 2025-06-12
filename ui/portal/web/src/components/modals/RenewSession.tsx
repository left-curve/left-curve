import { forwardRef, useEffect } from "react";
import { useApp } from "~/hooks/useApp";

import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "~/paraglide/messages";

import { Button, IconKey } from "@left-curve/applets-kit";
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
    <div className="flex flex-col bg-bg-primary-rice rounded-xl relative max-w-[400px]">
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
