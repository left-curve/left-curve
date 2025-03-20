import { useWizard } from "@left-curve/applets-kit";
import { useMutation } from "@tanstack/react-query";

import { AuthOptions } from "../auth/AuthOptions";

import {
  createWebAuthnCredential,
  ethHashMessage,
  secp256k1RecoverPubKey,
} from "@left-curve/dango/crypto";
import { encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { getNavigatorOS, getRootDomain } from "@left-curve/dango/utils";
import { useConnectors } from "@left-curve/store-react";

import { createKeyHash } from "@left-curve/dango";
import type { Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "@left-curve/store-react/types";
import type React from "react";
import { useToast } from "../foundation/Toast";

export const SignupCredentialStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const connectors = useConnectors();
  const { toast } = useToast();

  const { isPending, mutateAsync: createCredential } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");
        const challenge = "Please sign this message to confirm your identity.";
        const { key, keyHash } = await (async () => {
          if (connectorId === "passkey") {
            const { id, getPublicKey } = await createWebAuthnCredential({
              challenge: encodeUtf8(challenge),
              user: {
                name: `${getNavigatorOS()} ${new Date().toLocaleString()}`,
              },
              rp: {
                name: window.document.title,
                id: getRootDomain(window.location.hostname),
              },
              authenticatorSelection: {
                residentKey: "preferred",
                requireResidentKey: false,
                userVerification: "preferred",
              },
            });

            const publicKey = await getPublicKey();
            const key: Key = { secp256r1: encodeBase64(publicKey) };
            const keyHash = createKeyHash({ credentialId: id });

            return { key, keyHash };
          }

          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();

          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
          const signature = await provider.request({
            method: "personal_sign",
            params: [challenge, controllerAddress],
          });

          const pubKey = await secp256k1RecoverPubKey(ethHashMessage(challenge), signature, true);

          const key: Key = { secp256k1: encodeBase64(pubKey) };
          const keyHash = createKeyHash({ pubKey });

          return { key, keyHash };
        })();
        setData({ key, keyHash, connectorId });
        nextStep();
      } catch (err) {
        toast.error({ title: "Couldn't complete the request" });
        console.log(err);
      }
    },
  });

  return <AuthOptions action={createCredential} isPending={isPending} mode="signup" />;
};
