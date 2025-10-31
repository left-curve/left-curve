import { useSigninWithDesktop } from "@left-curve/store";
import { forwardRef, useEffect } from "react";

import { Spinner, useApp } from "@left-curve/applets-kit";

import { WEBRTC_URI } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { useNavigate } from "@tanstack/react-router";

export const SignWithDesktopFromNativeCamera = forwardRef<
  unknown,
  { socketId: string; navigate: ReturnType<typeof useNavigate> }
>(({ socketId, navigate }, _ref) => {
  const { toast, hideModal } = useApp();

  const { mutateAsync: connectWithDesktop } = useSigninWithDesktop({
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
      <div className="flex flex-col items-center justify-center gap-2 p-4">
        <Spinner size="lg" color="blue" />
        <p className="diatype-m-bold text-center text-ink-tertiary-500">
          {m["signin.authorizeInDesktop"]()}
        </p>
      </div>
    </div>
  );
});
