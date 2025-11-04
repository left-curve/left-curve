import { useAccount, useConnectorClient, useMessageExchanger } from "@left-curve/store";
import { forwardRef, useState } from "react";

import { IconButton, IconClose, IconMobile, QRCode, useApp } from "@left-curve/applets-kit";

import { decodeBase64 } from "@left-curve/dango/encoding";
import { captureException } from "@sentry/react";
import { WS_URI } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { JsonValue } from "@left-curve/dango/types";

export const QRConnect = forwardRef((_props, _ref) => {
  const { toast, hideModal } = useApp();
  const { data: signingClient } = useConnectorClient();
  const { username } = useAccount();

  const [isLoadingCredential, setIsLoadingCredential] = useState(false);
  const { messageExchanger, isLoading } = useMessageExchanger({
    url: WS_URI,
    subscribe: async (msg, exchanger) => {
      const { id, type, message } = msg;
      if (!signingClient || isLoadingCredential || type !== "create-session") return;
      try {
        setIsLoadingCredential(true);

        const { expireAt, publicKey } = message as { expireAt: number; publicKey: string };

        const response = await signingClient.createSession({
          expireAt,
          pubKey: decodeBase64(publicKey),
        });

        exchanger.sendMessage({ id, message: { data: { ...response, username } } });
        toast.success({
          title: "Connection established",
          description: null,
        });
        hideModal();
      } catch (error) {
        captureException(error);
        console.error("Error creating session: ", error);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.mobileSessionAborted"](),
        });
        hideModal();
        exchanger.sendMessage({
          id,
          message: { error: error instanceof Error ? error.message : (error as JsonValue) },
        });
      } finally {
        setIsLoadingCredential(false);
      }
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
        <div className="w-12 h-12 rounded-full bg-primitives-blue-light-100 flex items-center justify-center text-primitives-blue-light-600">
          <IconMobile />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="h4-bold text-ink-primary-900">{m["modals.qrconnect.title"]()}</h3>
          <p className="text-ink-tertiary-500 diatype-m-regular">
            {m["modals.qrconnect.description"]()}
          </p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-outline-secondary-gray my-2" />
      <div className="flex justify-center items-center p-8">
        <QRCode
          isLoading={isLoading || isLoadingCredential}
          data={
            messageExchanger
              ? `${document.location.origin}/signin?socketId=${messageExchanger.getSocketId()}`
              : undefined
          }
        />
      </div>
    </div>
  );
});
