import { useMutation } from "@tanstack/react-query";
import { forwardRef } from "react";

import { IconButton, IconClose, useApp } from "@left-curve/applets-kit";

import { useConnectors, usePublicClient, useSessionKey } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";

import type { ModalRef } from "./RootModal";

import { AuthOptions } from "../auth/AuthOptions";

interface ConnectWalletProps {
  onSuccess?: () => void;
  onError?: () => void;
}

export const ConnectWallet = forwardRef<ModalRef, ConnectWalletProps>(({ onSuccess }, ref) => {
  const { hideModal } = useApp();

  const { toast, settings } = useApp();
  const { createSessionKey } = useSessionKey();
  /* const { nextStep, setData } = useWizard(); */
  const { useSessionKey: session } = settings;
  const connectors = useConnectors();
  const publicClient = usePublicClient();

  const { isPending, mutateAsync: signInWithCredential } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");

        if (session) {
          const signingSession = await createSessionKey(
            { connector, expireAt: Date.now() + DEFAULT_SESSION_EXPIRATION },
            { setSession: false },
          );
          const usernames = await publicClient.forgotUsername({ keyHash: signingSession.keyHash });

          /* setData({ usernames, connectorId, signingSession }); */
        } else {
          const keyHash = await connector.getKeyHash();
          const usernames = await publicClient.forgotUsername({ keyHash });
          /* setData({ usernames, connectorId, keyHash }); */
        }
        onSuccess?.();
      } catch (err) {
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
        console.log(err);
      }
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray rounded-xl relative gap-4 w-full md:max-w-[25rem] p-6 pt-4">
      <IconButton
        className="hidden md:block absolute right-5 top-5"
        variant="link"
        onClick={() => [hideModal()]}
      >
        <IconClose />
      </IconButton>

      <div className="md:flex flex-col gap-4 md:pt-3 hidden">
        <p className="text-ink-primary-900 diatype-lg-medium">Connect Wallet</p>
      </div>

      <div className="flex flex-col gap-3 items-center">
        <AuthOptions action={signInWithCredential} isPending={isPending} mode="signin" />
      </div>
    </div>
  );
});
