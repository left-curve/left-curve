import { useSigninWithDesktop } from "@left-curve/store";
import { captureException } from "@sentry/react";
import { forwardRef, useEffect } from "react";
import { useApp } from "~/hooks/useApp";

import { Spinner } from "@left-curve/applets-kit";
import { Scanner } from "@yudiel/react-qr-scanner";
import { toast } from "../foundation/Toast";

import { WEBRTC_URI } from "~/constants";
import { m } from "~/paraglide/messages";

export const SignWithDesktop = forwardRef<unknown, { socketId: string }>(({ socketId }, _ref) => {
  const { router, hideModal } = useApp();

  const { mutateAsync: connectWithDesktop, isPending } = useSigninWithDesktop({
    url: WEBRTC_URI,
    mutation: {
      onSuccess: () => {
        router.navigate({ to: "/" });
        hideModal();
      },
      onError: (err) => {
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSignInWithDesktop"](),
        });
        console.error(err);
        captureException(err);
      },
    },
  });

  useEffect(() => {
    if (socketId) connectWithDesktop({ socketId });
  }, []);

  return (
    <div className="flex flex-col h-full bg-bg-primary-rice items-center justify-center gap-2">
      {isPending ? (
        <div className="flex flex-col items-center justify-center gap-2 p-4">
          <Spinner size="lg" color="pink" />
          <p className="diatype-m-bold text-center">{m["signin.authorizeInDesktop"]()}</p>
        </div>
      ) : (
        <>
          <div className="flex justify-center items-center py-12">
            <p className="diatype-m-medium text-gray-400 p-4 text-center">
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
            classNames={{ container: "qr-container", video: "bg-bg-primary-rice" }}
          />
          <div className="py-20 flex items-center justify-center">
            <p className="text-gray-400 diatype-m-medium" />
          </div>
        </>
      )}
    </div>
  );
});
