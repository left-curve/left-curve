import {
  Checkbox,
  ExpandOptions,
  IconPasskey,
  IconQR,
  useMediaQuery,
  useWizard,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useConnectors,
  usePublicClient,
  useSessionKey,
  useSignin,
} from "@left-curve/store";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { useApp } from "~/hooks/useApp";

import { Button, IconLeft, ResizerContainer } from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { toast } from "../foundation/Toast";
import { Modals } from "../modals/RootModal";
import { AuthCarousel } from "./AuthCarousel";
import { AuthOptions } from "./AuthOptions";
import { UsernamesList } from "./UsernamesList";

import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "~/paraglide/messages";

import type { Hex, SigningSession, Username } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-svh xl:h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer layoutId="signin" className="w-full max-w-[22.5rem]">
          {children}
        </ResizerContainer>
      </div>
      <AuthCarousel />
    </div>
  );
};

const CredentialStep: React.FC = () => {
  const { settings, changeSettings, showModal } = useApp();
  const { createSessionKey } = useSessionKey();
  const { nextStep, setData } = useWizard();
  const { useSessionKey: session } = settings;
  const connectors = useConnectors();
  const navigate = useNavigate();
  const publicClient = usePublicClient();

  const { isMd } = useMediaQuery();

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

          setData({ usernames, connectorId, signingSession });
        } else {
          const keyHash = await connector.getKeyHash();
          const usernames = await publicClient.forgotUsername({ keyHash });
          setData({ usernames, connectorId, keyHash });
        }
        nextStep();
      } catch (err) {
        toast.error({ title: m["errors.failureRequest"]() });
        console.log(err);
      }
    },
  });

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-2 lg:px-0">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-btn-shadow-gradient"
        />
        <h1 className="h2-heavy">{m["common.signin"]()}</h1>
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        {isMd ? (
          <AuthOptions action={signInWithCredential} isPending={isPending} mode="signin" />
        ) : (
          <Button
            fullWidth
            onClick={() => signInWithCredential("passkey")}
            isLoading={isPending}
            className="gap-2"
          >
            <IconPasskey className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signWithPasskey"]({ action: "signin" })}</p>
          </Button>
        )}

        {isMd ? (
          <Button as={Link} fullWidth variant="secondary" to="/" isDisabled={isPending}>
            {m["signin.continueWithoutSignin"]()}
          </Button>
        ) : (
          <Button
            fullWidth
            onClick={() => showModal(Modals.SignWithDesktop)}
            className="gap-2"
            variant="secondary"
          >
            <IconQR className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
          </Button>
        )}
        <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
          <div className="flex items-center gap-2 flex-col">
            <Checkbox
              size="md"
              label={m["common.signinWithSession"]()}
              checked={session}
              onChange={(v) => changeSettings({ useSessionKey: v })}
            />
          </div>
        </ExpandOptions>
      </div>

      {isMd ? (
        <div className="flex justify-center items-center">
          <p>{m["signin.noAccount"]()}</p>
          <Button variant="link" onClick={() => navigate({ to: "/signup" })} isDisabled={isPending}>
            {m["common.signup"]()}
          </Button>
        </div>
      ) : (
        <Button as={Link} fullWidth variant="link" to="/">
          {m["signin.continueWithoutSignin"]()}
        </Button>
      )}
    </div>
  );
};

const UsernameStep: React.FC = () => {
  const { data, previousStep } = useWizard<{
    usernames: Username[];
    keyHash?: Hex;
    signingSession?: SigningSession;
    connectorId: string;
  }>();
  const navigate = useNavigate();
  const { usernames, connectorId, keyHash, signingSession } = data;

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    session: signingSession,
    mutation: {
      onSuccess: () => {
        navigate({ to: "/" });
      },
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
      },
    },
  });

  const existUsernames = usernames.length;

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-btn-shadow-gradient"
        />
        {existUsernames ? (
          <>
            <h1 className="h2-heavy">{m["signin.usernamesFound"]()}</h1>
            <p className="text-gray-500 diatype-m-medium">{m["signin.chooseCredential"]()}</p>
          </>
        ) : (
          <>
            <h1 className="h2-heavy">{m["signin.noUsernamesFound"]()}</h1>
            <p className="text-gray-500 diatype-m-medium">{m["signin.noUsernameMessage"]()}</p>
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
          <Button variant="link" onClick={previousStep} isLoading={isPending}>
            <IconLeft className="w-[22px] h-[22px]" />
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

export const Signin = Object.assign(Container, {
  Username: UsernameStep,
  Credential: CredentialStep,
});
