import { useWizard } from "@left-curve/applets-kit";
import { useLogin, useLoginWithDesktop } from "@left-curve/store-react";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { toast } from "../foundation/Toast";

import { Button, IconPasskey, IconQR } from "@left-curve/applets-kit";
import { QRScan } from "./QRScan";

import { m } from "~/paraglide/messages";

import type React from "react";

export const AuthMobile: React.FC = () => {
  const navigate = useNavigate();
  const { data } = useWizard<{ username: string }>();
  const [isScannerVisible, setScannerVisibility] = useState(false);

  const { username } = data;

  const { mutateAsync: connectWithPasskey, isPending } = useLogin({
    username,
    mutation: {
      onSuccess: () => navigate({ to: "/" }),
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigingIn"](),
        });
      },
    },
  });

  const { mutateAsync: connectWithDesktop } = useLoginWithDesktop({
    url: import.meta.env.PUBLIC_WEBRTC_URI,
    username,
    mutation: {
      onSuccess: () => navigate({ to: "/" }),
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigingIn"](),
        });
      },
    },
  });

  return (
    <>
      {isScannerVisible ? (
        <QRScan
          onScan={(socketId) => connectWithDesktop({ socketId })}
          isVisisble={isScannerVisible}
          onClose={() => setScannerVisibility(false)}
        />
      ) : null}
      <div className="flex flex-col gap-4 w-full">
        <Button
          fullWidth
          onClick={() => connectWithPasskey({ connectorId: "passkey" })}
          isLoading={isPending}
          className="gap-2"
        >
          <IconPasskey className="w-6 h-6" />
          <p className="min-w-20"> {m["common.signWithPasskey"]({ action: "signin" })}</p>
        </Button>
        <Button
          fullWidth
          onClick={() => setScannerVisibility(true)}
          className="gap-2"
          variant="secondary"
        >
          <IconQR className="w-6 h-6" />
          <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
        </Button>
      </div>
    </>
  );
};
