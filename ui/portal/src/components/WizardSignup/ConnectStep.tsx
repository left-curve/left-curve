import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useWizard } from "@dango/shared";
import { useAccount, useConfig, useConnectors, usePublicClient } from "@leftcurve/react";
import { useMutation } from "@tanstack/react-query";

import {
  createWebAuthnCredential,
  ethHashMessage,
  secp256k1RecoverPubKey,
} from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { computeAddress, createAccountSalt, createKeyHash } from "@leftcurve/sdk";
import { getNavigatorOS, getRootDomain, sleep } from "@leftcurve/utils";

import { DangoButton, Select, SelectItem } from "@dango/shared";

import {
  AccountType,
  type Address,
  ConnectionStatus,
  type EIP1193Provider,
  type Key,
  KeyAlgo,
} from "@leftcurve/types";

export const ConnectStep: React.FC = () => {
  const navigate = useNavigate();

  const config = useConfig();
  const client = usePublicClient();

  const { status } = useAccount();
  const connectors = useConnectors();

  const [connectorId, setConnectorId] = useState<string>("Passkey");

  const { data } = useWizard<{ username: string }>();
  const { username } = data;

  const {
    mutateAsync: createAccount,
    isPending,
    isError,
  } = useMutation({
    mutationFn: async () => {
      try {
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
            const keyHash = createKeyHash({ credentialId: id, keyAlgo: KeyAlgo.Secp256r1 });

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

            const pubKey = await secp256k1RecoverPubKey(ethHashMessage(challenge), signature, true);

            const key: Key = { secp256k1: encodeBase64(pubKey) };
            const keyHash = createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });

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
        // TODO: Do pooling instead of sleep to check account creation
        await sleep(1000);
        await connector.connect({ username, chainId: config.chains[0].id });
      } catch (err) {
        console.log(err);
        throw err;
      }
    },
  });

  useEffect(() => {
    if (status !== ConnectionStatus.Connected) return;
    navigate("/");
  }, [navigate, status]);

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <DangoButton fullWidth onClick={() => createAccount()} isLoading={isPending}>
        Signup with {connectorId}
      </DangoButton>
      {isError ? (
        <p className="text-typography-rose-600 text-center text-xl">
          We couldn't complete the request
        </p>
      ) : null}
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
        isDisabled={isPending}
      >
        Already have an account?
      </DangoButton>
    </div>
  );
};
