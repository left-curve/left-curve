import { IconLeft, ResizerContainer, useUsernames, useWizard } from "@left-curve/applets-kit";
import {
  useAccount,
  useChainId,
  useConnectors,
  usePublicClient,
  useSignin,
} from "@left-curve/store";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { toast } from "../foundation/Toast";

import { Button } from "@left-curve/applets-kit";
import { AuthOptions } from "./AuthOptions";

import { m } from "~/paraglide/messages";

import type { Hex, Key, Username } from "@left-curve/dango/types";
import type React from "react";
import { AuthCarousel } from "./AuthCarousel";

import { UsernamesList } from "./UsernamesList";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { useApp } from "~/hooks/useApp";

const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
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
  const publicClient = usePublicClient();

  const { isPending, mutateAsync: createCredential } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");
        const keyHash = await connector.getKeyHash();
        const usernames = await publicClient.forgotUsername({ keyHash });
        setData({ usernames, connectorId, keyHash });
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
        <h1 className="h2-heavy">{m["forgotUsername.greetings"]()}</h1>
        <p className="text-gray-500 diatype-m-medium">{m["forgotUsername.chooseCredential"]()}</p>
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
  const { data, previousStep, done } = useWizard<{
    usernames: Username[];
    keyHash: Hex;
    connectorId: string;
  }>();
  const navigate = useNavigate();
  const { usernames, connectorId, keyHash } = data;
  const { settings } = useApp();
  const { addUsername } = useUsernames();

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    sessionKey: settings.useSessionKey && { expireAt: Date.now() + DEFAULT_SESSION_EXPIRATION },
    mutation: {
      onSuccess: (username) => {
        navigate({ to: "/" });
        addUsername(username);
        done();
      },
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
        previousStep();
      },
    },
  });

  const existUsernames = usernames.length;

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
            <h1 className="h2-heavy">{m["forgotUsername.usernamesFound"]()}</h1>
            <p className="text-gray-500 diatype-m-medium">
              {m["forgotUsername.chooseCredentialFoundUsername"]()}
            </p>
          </>
        ) : (
          <>
            <h1 className="h2-heavy">{m["forgotUsername.noUsernamesFound"]()}</h1>
            <p className="text-gray-500 diatype-m-medium">
              {m["forgotUsername.noUsernameMessage"]()}
            </p>
          </>
        )}
      </div>
      {existUsernames ? (
        <div className="flex flex-col gap-4 w-full items-center">
          <UsernamesList
            usernames={usernames.reduce((acc, u) => {
              acc[u] = {};
              return acc;
            }, Object.create({}))}
            showArrow={true}
            onClick={(username) => connectWithConnector({ username, connectorId, keyHash })}
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
