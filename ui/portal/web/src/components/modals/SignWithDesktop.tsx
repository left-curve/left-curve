import { useSigninWithDesktop } from "@left-curve/store";
import { forwardRef, useEffect } from "react";

import { Spinner, useApp } from "@left-curve/applets-kit";
import { Scanner } from "@yudiel/react-qr-scanner";

import { WEBRTC_URI } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { useNavigate } from "@tanstack/react-router";

export const SignWithDesktop = forwardRef<
  unknown,
  { socketId: string; navigate: ReturnType<typeof useNavigate> }
>(({ socketId, navigate }, _ref) => {
  const { toast, hideModal } = useApp();

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
        navigate({ to: "/" });
        hideModal();
      },
    },
  });

  useEffect(() => {
    if (socketId) connectWithDesktop({ socketId });
  }, []);

  return (
    <div className="flex flex-col h-full bg-surface-primary-rice items-center justify-center gap-2">
      {isPending ? (
        <div className="flex flex-col items-center justify-center gap-2 p-4">
          <Spinner size="lg" color="pink" />
          <p className="diatype-m-bold text-center">{m["signin.authorizeInDesktop"]()}</p>
        </div>
      ) : (
        <>
          <div className="flex justify-center items-center py-12">
            <p className="diatype-m-medium text-tertiary-500 p-4 text-center">
              {m["signin.qrInstructions"]({ domain: window.location.hostname })}
            </p>
          </div>
          <Scanner
            onScan={([{ rawValue }]) => {
              const socketId = rawValue.split("socketId=")[1];
              if (!socketId) return;
              connectWithDesktop({ socketId });
            }}
            allowMultiple={false}
            components={{ audio: false }}
            formats={["qr_code"]}
            classNames={{ container: "qr-container", video: "bg-surface-primary-rice" }}
          />
          <div className="py-20 flex items-center justify-center">
            <p className="text-tertiary-500 diatype-m-medium" />
          </div>
        </>
      )}
    </div>
  );
});
