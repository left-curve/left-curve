import {
  Checkbox,
  ExpandOptions,
  IconAlert,
  IconLeft,
  ResizerContainer,
  Stepper,
  useUsernames,
  useWizard,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useConfig,
  useConnectors,
  usePublicClient,
  useSignin,
} from "@left-curve/store";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { computeAddress, createAccountSalt } from "@left-curve/dango";
import { createKeyHash } from "@left-curve/dango";
import { createWebAuthnCredential } from "@left-curve/dango/crypto";
import { encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { getNavigatorOS, getRootDomain } from "@left-curve/dango/utils";

import { registerUser } from "@left-curve/dango/actions";
import { AccountType } from "@left-curve/dango/types";
import { wait } from "@left-curve/dango/utils";
import { toast } from "../foundation/Toast";

import {
  Button,
  CheckCircleIcon,
  Input,
  Spinner,
  XCircleIcon,
  useInputs,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { AuthOptions } from "./AuthOptions";

import { m } from "~/paraglide/messages";

import type { Address, Hex, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "@left-curve/store/types";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { AuthCarousel } from "./AuthCarousel";

import { captureException } from "@sentry/react";
import { UsernamesList } from "./UsernamesList";

const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep, data } = useWizard<{ username: string }>();
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <>
      <div className="h-screen w-screen flex items-center justify-center">
        <div className="flex items-center justify-center flex-1">
          <ResizerContainer layoutId="signup" className="w-full max-w-[22.5rem]">
            <div className="flex items-center justify-center gap-8 px-4 lg:px-0 flex-col w-full">
              {children}
            </div>
          </ResizerContainer>
        </div>
        <AuthCarousel />
      </div>
    </>
  );
};

const Credential: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const connectors = useConnectors();
  const navigate = useNavigate();

  const { isPending, mutateAsync: createCredential } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        /* const connector = connectors.find((c) => c.id === connectorId);
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
            const keyHash = createKeyHash(id);

            return { key, keyHash };
          }

          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();

          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          const addressLowerCase = controllerAddress.toLowerCase() as Address;

          const key: Key = { ethereum: addressLowerCase };
          const keyHash = createKeyHash(addressLowerCase);

          return { key, keyHash };
        })(); */
        /* setData({ key, keyHash, connectorId, seed: Math.floor(Math.random() * 0x100000000) }); */
        nextStep();
      } catch (err) {
        toast.error({ title: "Couldn't complete the request" });
        console.log(err);
      }
    },
  });

  return (
    <div className="flex flex-col gap-6 w-full items-center">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-btn-shadow-gradient"
        />
        <h1 className="h2-heavy">Hi there</h1>
        <p className="text-gray-500 diatype-m-medium">
          Choose any of the credentials that have been associated with your username.
        </p>
      </div>
      <AuthOptions action={createCredential} isPending={isPending} mode="signup" expanded={true} />
      <Button variant="link" onClick={() => navigate({ to: "/signin" })}>
        <IconLeft className="w-[22px] h-[22px] text-blue-500" />
        <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
      </Button>
    </div>
  );
};

const AvailableUsernames: React.FC = () => {
  const { nextStep, data, setData, previousStep } = useWizard<{
    key: Key;
    keyHash: Hex;
    connectorId: string;
    seed: number;
    username: string;
  }>();
  const navigate = useNavigate();

  const fillUsername = (username: string) => {
    //setData({ username, sessionKey: useSessionKey });
    setData({ username });
    nextStep();
  };

  const usernames = {
    iris: {},
    dango: {},
  };

  const { isPending, mutateAsync: createAccount } = useMutation({
    mutationFn: async () => {
      try {
        // Select username & continue
        nextStep();
      } catch (err) {
        toast.error({ title: "Couldn't complete the request" });
        console.log(err);
      }
    },
  });

  const existUsernames = Object.keys(usernames);

  return (
    <div className="flex flex-col gap-6 w-full">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-btn-shadow-gradient"
        />
        {existUsernames ? (
          <>
            <h1 className="h2-heavy">Usernames found</h1>
            <p className="text-gray-500 diatype-m-medium">
              Choose any of the credentials that have been associated with your username.
            </p>
          </>
        ) : (
          <>
            <h1 className="h2-heavy">No username found</h1>
            <p className="text-gray-500 diatype-m-medium">
              We could not find any username associated with the credential you connected.
            </p>
          </>
        )}
      </div>
      {existUsernames ? (
        <div className="flex flex-col gap-4 w-full items-center">
          <UsernamesList
            usernames={usernames}
            showArrow={true}
            onClick={(username) => fillUsername(username)}
          />
          <Button variant="link" onClick={previousStep}>
            <IconLeft className="w-[22px] h-[22px] text-blue-500" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      ) : (
        <Button variant="link" onClick={previousStep}>
          <IconLeft className="w-[22px] h-[22px] text-blue-500" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      )}
    </div>
  );
};

export const ForgotUsername = Object.assign(Container, {
  Credential,
  AvailableUsernames,
});
