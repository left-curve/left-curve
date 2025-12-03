import { useSigninWithDesktop } from "@left-curve/store";
import { forwardRef } from "react";

import { QRCodeReader, Spinner, useApp } from "@left-curve/applets-kit";

import { WS_URI } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const SignWithDesktop = forwardRef((_, _ref) => {
  const { toast, hideModal } = useApp();

  const { mutateAsync: connectWithDesktop, isPending } = useSigninWithDesktop({
    url: WS_URI,
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSignInWithDesktop"](),
        }),
    },
    mutation: {
      onSuccess: () => {
        hideModal();
      },
    },
  });

  return (
    <div className="flex flex-col h-full bg-surface-primary-rice items-center justify-center gap-2">
      {isPending ? (
        <div className="flex flex-col items-center justify-center gap-2 p-4">
          <Spinner size="lg" color="blue" />
          <p className="diatype-m-bold text-center text-ink-tertiary-500">
            {m["signin.authorizeInDesktop"]()}
          </p>
        </div>
      ) : (
        <div className="h-full w-full">
          <QRCodeReader onScan={(socketId) => connectWithDesktop({ socketId })} />
        </div>
      )}
    </div>
  );
});
