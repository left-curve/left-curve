import { useSigninWithDesktop } from "@left-curve/store";
import { forwardRef } from "react";

import { Spinner, useApp } from "@left-curve/applets-kit";
import { QRScan } from "./QRScan";

import { WEBRTC_URI } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const SignWithDesktop = forwardRef((_, _ref) => {
  const { toast, hideModal, navigate } = useApp();

  const { mutateAsync: connectWithDesktop, isPending } = useSigninWithDesktop({
    url: WEBRTC_URI,
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSignInWithDesktop"](),
        }),
    },
    mutation: {
      onSuccess: () => {
        navigate("/");
        hideModal();
      },
    },
  });

  return (
    <div className="flex flex-col h-full bg-surface-primary-rice items-center justify-center gap-2">
      {isPending ? (
        <div className="flex flex-col items-center justify-center gap-2 p-4">
          <Spinner size="lg" color="pink" />
          <p className="diatype-m-bold text-center">{m["signin.authorizeInDesktop"]()}</p>
        </div>
      ) : (
        <div className="h-full w-full">
          <QRScan onScan={(scannedSocketId) => connectWithDesktop({ socketId: scannedSocketId })} />
        </div>
      )}
    </div>
  );
});
