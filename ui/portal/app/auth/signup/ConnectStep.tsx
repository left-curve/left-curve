"use client";

import { useRouter } from "next/router";
import { useEffect, useState } from "react";

import { ArrowSelectorIcon, ConnectorButtonOptions, twMerge, useWizard } from "@dango/shared";
import { useAccount, useConfig, useConnectors, usePublicClient } from "@leftcurve/react";
import { useMutation } from "@tanstack/react-query";

import {
  createWebAuthnCredential,
  ethHashMessage,
  secp256k1RecoverPubKey,
} from "@leftcurve/crypto";
import { encodeBase64, encodeUtf8 } from "@leftcurve/encoding";
import { computeAddress, createAccountSalt, createKeyHash } from "@leftcurve/sdk";
import { AccountType, ConnectionStatus, KeyAlgo } from "@leftcurve/types";
import { getNavigatorOS, getRootDomain, wait } from "@leftcurve/utils";

import { Button } from "@dango/shared";

import type { EIP1193Provider, Key } from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export const ConnectStep: React.FC = () => {
  const [connectorLoading, setConnectorLoading] = useState<string>();
  const { push: navigate } = useRouter();

  const config = useConfig();
  const client = usePublicClient();

  const { status } = useAccount();
  const connectors = useConnectors();

  const { data } = useWizard<{ username: string }>();
  const { username } = data;

  const { mutateAsync: createAccount, isError } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        setConnectorLoading(connectorId);
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
            const keyHash = createKeyHash({ credentialId: id, keyAlgo: KeyAlgo.Secp256r1 });

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
          const keyHash = createKeyHash({ pubKey, keyAlgo: KeyAlgo.Secp256k1 });

          return { key, keyHash };
        })();

        const { addresses } = await client.getAppConfig<DangoAppConfigResponse>();
        const accountCodeHash = await client.getAccountTypeCodeHash({
          accountType: AccountType.Spot,
        });

        const salt = createAccountSalt({ key, keyHash, username });
        const address = computeAddress({
          deployer: addresses.accountFactory,
          codeHash: accountCodeHash,
          salt,
        });

        const response = await fetch("https://mock-ibc.left-curve.workers.dev", {
          method: "POST",
          body: JSON.stringify({ address }),
        });
        if (!response.ok) throw new Error("error: failed to send funds");

        await client.registerUser({ key, keyHash, username });
        // TODO: Do pooling instead of wait to check account creation
        await wait(1000);
        await connector.connect({ username, chainId: config.chains[0].id });
      } catch (err) {
        console.log(err);
        throw err;
      } finally {
        setConnectorLoading(undefined);
      }
    },
  });

  useEffect(() => {
    if (status !== ConnectionStatus.Connected) return;
    navigate("/");
  }, [navigate, status]);

  const [showOtherSignup, setShowOtherSignup] = useState(false);

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <Button
        fullWidth
        onClick={() => createAccount("passkey")}
        isLoading={connectorLoading === "passkey"}
      >
        Signup with Passkey
      </Button>
      {isError ? (
        <p className="text-typography-rose-600 text-center text-xl">
          We couldn't complete the request
        </p>
      ) : null}
      <div className="flex items-center justify-center">
        <span className="h-[1px] w-full flex-1 bg-typography-purple-400" />
        <button
          className="px-3 flex gap-2 items-center justify-center"
          type="button"
          onClick={() => setShowOtherSignup((current) => !current)}
        >
          <span>Other wallets </span>
          <span>
            <ArrowSelectorIcon
              className={twMerge("w-4 h-4 transition-all", { "rotate-180": showOtherSignup })}
            />
          </span>
        </button>
        <span className="h-[1px] w-full flex-1 bg-typography-purple-400" />
      </div>
      <div
        className={twMerge("transition-all w-full flex flex-col gap-2 h-0 overflow-hidden", {
          "h-fit": showOtherSignup,
        })}
      >
        <ConnectorButtonOptions
          mode="signup"
          connectors={connectors}
          selectedConnector={connectorLoading}
          onClick={createAccount}
        />
      </div>
    </div>
  );
};
