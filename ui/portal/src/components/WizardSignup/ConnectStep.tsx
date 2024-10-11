import { DangoButton, Select, SelectItem, useWizard } from "@dango/shared";
import { createWebAuthnCredential, ethHashMessage, recoverPublicKey } from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { useConnectors, usePublicClient } from "@leftcurve/react";
import { computeAddress, createAccountSalt, createKeyHash } from "@leftcurve/sdk";
import { getNavigatorOS, getRootDomain } from "@leftcurve/utils";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { AccountType, type Address, type EIP1193Provider, type Key } from "@leftcurve/types";

export const ConnectStep: React.FC = () => {
  const { data } = useWizard<{ username: string }>();
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [connectorId, setConnectorId] = useState<string>("Passkey");
  const connectors = useConnectors();
  const client = usePublicClient();
  const navigate = useNavigate();
  const { username } = data;

  const onSubmit = async () => {
    try {
      setError(null);
      setIsLoading(true);
      const connector = connectors.find((c) => c.id === connectorId.toLowerCase());
      if (!connector) throw new Error("error: missing connector");
      const challenge = "Please sign this message to confirm your identity.";
      const { key, keyHash } = await (async () => {
        if (connectorId === "Passkey") {
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

      const factoryAddr = await client.getAppConfig<Address>({ key: "account_factory" });
      const accountCodeHash = await client.getAccountTypeCodeHash({
        accountType: AccountType.Spot,
      });

      const salt = createAccountSalt({ key, keyHash, username });
      const address = computeAddress({ deployer: factoryAddr, codeHash: accountCodeHash, salt });

      const response = await fetch("https://mock-ibc.left-curve.workers.dev", {
        method: "POST",
        body: JSON.stringify({ address }),
      });
      if (!response.ok) throw new Error("error: failed to send funds");

      await client.registerUser({ key, keyHash, username });

      navigate("/auth/login");
    } catch (err) {
      console.log(err);
      setIsLoading(false);
      setError("something went wrong");
    }
  };

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <DangoButton fullWidth onClick={onSubmit} isLoading={isLoading}>
        Signup with {connectorId}
      </DangoButton>
      {error ? <p className="text-typography-rose-600 text-center text-xl">{error}</p> : null}
      <Select
        label="login-methods"
        placeholder="Alternative sign up methods"
        defaultSelectedKey={connectorId}
        onSelectionChange={(key) => setConnectorId(key.toString())}
      >
        <SelectItem key="Passkey">Passkey</SelectItem>
        <SelectItem key="Metamask">Metamask</SelectItem>
      </Select>
      <DangoButton
        as={Link}
        to="/auth/login"
        variant="ghost"
        color="sand"
        className="text-lg"
        isDisabled={isLoading}
      >
        Already have an account?
      </DangoButton>
    </div>
  );
};
