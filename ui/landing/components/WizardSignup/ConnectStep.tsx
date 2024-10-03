"use client";

import { DangoButton, Select, SelectItem, useWizard } from "@dango/shared";
import { createWebAuthnCredential, ethHashMessage, recoverPublicKey } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { useConnectors, usePublicClient } from "@leftcurve/react";
import { createKeyHash } from "@leftcurve/sdk";
import { getNavigatorOS } from "@leftcurve/utils";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type React from "react";
import { useState } from "react";

import type { EIP1193Provider, Key } from "@leftcurve/types";

export const ConnectStep: React.FC = () => {
  const { data } = useWizard<{ username: string }>();
  const [connectorId, setConnectorId] = useState<string>("Passkey");
  const connectors = useConnectors();
  const client = usePublicClient();
  const { push } = useRouter();
  const { username } = data;

  const onSubmit = async () => {
    const connector = connectors.find((c) => c.id === connectorId.toLowerCase());
    if (!connector) throw new Error("error: missing connector");
    const challenge = "Please sign this message to confirm your identity.";
    const { key, keyHash } = await (async () => {
      if (connectorId === "Passkey") {
        const { publicKey, id } = await createWebAuthnCredential({
          challenge: encodeUtf8(challenge),
          user: {
            name: `${getNavigatorOS()} ${new Date().toLocaleString()}`,
          },
          rp: {
            name: window.document.title,
            id: window.location.hostname,
          },
          authenticatorSelection: {
            residentKey: "preferred",
            requireResidentKey: false,
            userVerification: "preferred",
          },
        });

        const key: Key = { secp256r1: encodeBase64(publicKey) };
        const keyHash = createKeyHash({ credentialId: id });

        return { key, keyHash };
      }

      if (connectorId === "Metamask") {
        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();

        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const signature = await provider.request({
          method: "personal_sign",
          params: [challenge, controllerAddress],
        });

        const pubKey = await recoverPublicKey(ethHashMessage(challenge), signature, true);

        const key: Key = { secp256k1: encodeBase64(pubKey) };
        const keyHash = createKeyHash({ pubKey });

        return { key, keyHash };
      }

      throw new Error("error: invalid connector");
    })();
    // TODO: Fund the account in IBC-Transfer before registering the user
    await client.registerUser({ key, keyHash, username });
    push("/login");
  };

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <DangoButton fullWidth onClick={onSubmit}>
        Signup with {connectorId}
      </DangoButton>
      <Select
        label="login-methods"
        placeholder="Alternative sign up methods"
        defaultSelectedKey={connectorId}
        onSelectionChange={(key) => setConnectorId(key.toString())}
      >
        <SelectItem key="Passkey">Passkey</SelectItem>
        <SelectItem key="Metamask">Metamask</SelectItem>
      </Select>
      <DangoButton as={Link} href="/login" variant="ghost" color="sand" className="text-lg">
        Already have an account?
      </DangoButton>
    </div>
  );
};
