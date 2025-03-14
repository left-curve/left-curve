import {
  Button,
  IconLeft,
  IconPasskey,
  IconQR,
  useMediaQuery,
  useWizard,
} from "@left-curve/applets-kit";
import { useChainId, useConnectors, useDataChannel } from "@left-curve/store-react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { Secp256k1 } from "@left-curve/dango/crypto";
import { encodeBase64, encodeUtf8, serializeJson } from "@left-curve/dango/encoding";
import type { SessionResponse } from "@left-curve/dango/types";
import type React from "react";
import { useState } from "react";
import { m } from "~/paraglide/messages";
import { AuthOptions } from "../AuthOptions";
import { useToast } from "../Toast";
import { QRScan } from "./QRScan";

export const LoginCredentialStep: React.FC = () => {
  const connectors = useConnectors();
  const navigate = useNavigate();
  const { toast } = useToast();
  const { data, previousStep } = useWizard<{ username: string }>();
  const chainId = useChainId();
  const isMd = useMediaQuery("md");
  const [isScannerVisible, setScannerVisibility] = useState(false);

  const { username } = data;

  const { data: dataChannel } = useDataChannel({ url: import.meta.env.PUBLIC_WEBRTC_URI });

  const { mutateAsync: connectWithConnector, isPending } = useMutation({
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");
      try {
        await connector.connect({
          username,
          chainId,
          challenge: "Please sign this message to confirm your identity.",
        });
        navigate({ to: "/" });
      } catch (err) {
        console.error(err);
        toast.error({
          title: "Error",
          description: "Failed to connect to the selected credential.",
        });
        previousStep();
      }
    },
  });

  const { mutateAsync: connectWithDesktop } = useMutation({
    mutationFn: async (socketId: string) => {
      try {
        if (!dataChannel) throw new Error("error: missing dataChannel");
        await dataChannel.createPeerConnection(socketId);
        const keyPair = Secp256k1.makeKeyPair();
        const publicKey = keyPair.getPublicKey();
        const { authorization, keyHash, sessionInfo } =
          await dataChannel.sendAsyncMessage<SessionResponse>({
            type: "generate_session",
            message: {
              expireAt: +new Date(Date.now() + 1000 * 60 * 5),
              publicKey: encodeBase64(publicKey),
            },
          });

        const connector = connectors.find((connector) => connector.id === "session");
        if (!connector) throw new Error("error: missing connector");
        await connector.connect({
          username,
          chainId,
          challenge: encodeBase64(
            encodeUtf8(
              serializeJson({
                authorization,
                keyHash,
                sessionInfo,
                publicKey,
                privateKey: keyPair.privateKey,
              }),
            ),
          ),
        });

        navigate({ to: "/" });
      } catch (err) {
        console.error(err);
        toast.error({
          title: "Error",
          description: "Failed to connect with desktop",
        });
      }
    },
  });

  return (
    <>
      <div className="flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
        <div className="flex flex-col gap-7 items-center justify-center">
          <img
            src="./favicon.svg"
            alt="dango-logo"
            className="h-12 rounded-full shadow-btn-shadow-gradient"
          />
          <div className="flex flex-col gap-3 items-center justify-center text-center">
            <h1 className="h2-heavy">
              {m["common.hi"]()}, {username}
            </h1>
            <p className="text-gray-500 diatype-m-medium">{m["login.credential.description"]()}</p>
          </div>
        </div>
        {isMd ? (
          <AuthOptions action={connectWithConnector} isPending={isPending} mode="signin" />
        ) : (
          <div className="flex flex-col gap-4 w-full">
            {isScannerVisible ? (
              <QRScan
                onScan={connectWithDesktop}
                isVisisble={isScannerVisible}
                onClose={() => setScannerVisibility(false)}
              />
            ) : null}
            <Button
              fullWidth
              onClick={() => connectWithConnector("passkey")}
              isLoading={isPending}
              className="gap-2"
            >
              <IconPasskey className="w-6 h-6" />
              <p className="min-w-20"> {m["common.signWithPasskey"]({ action: "signin" })}</p>
            </Button>
            <Button
              fullWidth
              onClick={() => setScannerVisibility(true)}
              isLoading={isPending}
              className="gap-2"
              variant="secondary"
            >
              <IconQR className="w-6 h-6" />
              <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
            </Button>
          </div>
        )}
        <div className="flex items-center">
          <Button variant="link" onClick={() => previousStep()}>
            <IconLeft className="w-[22px] h-[22px] text-blue-500" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      </div>
    </>
  );
};
