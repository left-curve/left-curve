import { IconButton, IconClose, IconMobile, QRCode } from "@left-curve/applets-kit";
import { decodeBase64 } from "@left-curve/dango/encoding";
import { Actions } from "@left-curve/dango/utils";
import { useConnectorClient, useDataChannel } from "@left-curve/store-react";
import type React from "react";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { useToast } from "../Toast";

export const QRConnect: React.FC = () => {
  const [isLoadingCredential, setIsLoadingCredential] = useState(false);
  const { data: dataChannel, isLoading: isLoadingDataChannel } = useDataChannel({
    url: import.meta.env.PUBLIC_WEBRTC_URI,
  });
  const { data: signingClient } = useConnectorClient();
  const { hideModal } = useApp();
  const { toast } = useToast();

  dataChannel?.subscribe(async (m) => {
    if (!signingClient) return;

    try {
      const { id, type, message } = m;
      if (type !== Actions.GenerateSession || isLoadingCredential) return;
      setIsLoadingCredential(true);

      const { expireAt, publicKey } = message as { expireAt: number; publicKey: string };

      const response = await signingClient.createSession({
        expireAt,
        pubKey: decodeBase64(publicKey),
      });

      dataChannel.sendMessage({ id, message: response });
      toast.success({ title: "Connection established" });
      hideModal();
    } catch (error) {
    } finally {
      setIsLoadingCredential(false);
    }
  });

  return (
    <div className="flex flex-col bg-white-100 rounded-3xl relative">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-blue-100 flex items-center justify-center text-blue-600">
          <IconMobile />
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="h4-bold">{m["modals.qrconnect.title"]()}</h3>
          <p className="text-gray-500 diatype-m-regular">{m["modals.qrconnect.description"]()}</p>
        </div>
      </div>
      <span className="w-full h-[1px] bg-gray-100 my-2" />
      <div className="flex justify-center items-center p-8">
        <QRCode
          isLoading={isLoadingDataChannel || isLoadingCredential}
          data={dataChannel?.getSocketId()}
        />
      </div>
    </div>
  );
};
