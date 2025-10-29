import {
  IconQR,
  IconWallet,
  Modals,
  useApp,
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

import { Button, IconLeft, ResizerContainer } from "@left-curve/applets-kit";
import { AuthCarousel } from "./AuthCarousel";
import { UsernamesList } from "./UsernamesList";
import { AuthOptions } from "./AuthOptions";

import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { Hex, SigningSession, Username } from "@left-curve/dango/types";
import type React from "react";
import type { PropsWithChildren } from "react";
import { EmailCredential } from "./EmailCredential";
import { SocialCredential } from "./SocialCredential";
import { PasskeyCredential } from "./PasskeyCredential";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  const { isConnected } = useAccount();
  const { data, activeStep } = useWizard<{ view: string; email: string; usernames: unknown[] }>();
  const navigate = useNavigate();

  const { view, email, usernames } = data;

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-svh xl:h-screen w-screen flex items-center justify-center bg-surface-primary-rice text-ink-primary-900">
      <div className="flex items-center justify-center flex-1 min-w-fit">
        <ResizerContainer layoutId="signin" className="w-full max-w-[24.5rem]">
          <div className="flex flex-col gap-7 items-center justify-center w-full">
            <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
              <img
                src="./favicon.svg"
                alt="dango-logo"
                className="h-12 rounded-full shadow-account-card"
              />
              {activeStep === 0 ? (
                <>
                  <h1 className="h2-heavy">{m["common.signin"]()}</h1>
                  {view === "email" && email ? (
                    <p className="text-ink-tertiary-500">
                      {m["signin.sentVerificationCode"]()}
                      <span className="font-bold">{email}</span>
                    </p>
                  ) : null}
                  {view === "wallets" ? (
                    <p className="text-ink-tertiary-500 diatype-m-medium">
                      {m["signin.connectWalletToContinue"]()}
                    </p>
                  ) : null}
                </>
              ) : usernames.length ? (
                <>
                  <h1 className="h2-heavy">{m["signin.usernamesFound"]()}</h1>
                  <p className="text-ink-tertiary-500 diatype-m-medium">
                    {m["signin.chooseCredential"]()}
                  </p>
                </>
              ) : (
                <>
                  <h1 className="h2-heavy">{m["signin.noUsernamesFound"]()}</h1>
                  <p className="text-ink-tertiary-500 diatype-m-medium">
                    {m["signin.noUsernameMessage"]()}
                  </p>
                </>
              )}
            </div>
            {children}
          </div>
        </ResizerContainer>
      </div>
      <AuthCarousel />
    </div>
  );
};

const CredentialStep: React.FC = () => {
  const { isMd } = useMediaQuery();
  const navigate = useNavigate();
  const { toast, settings, showModal } = useApp();
  const { data, setData, nextStep, reset } = useWizard();
  const { createSessionKey } = useSessionKey();
  const connectors = useConnectors();
  const publicClient = usePublicClient();
  const { useSessionKey: session } = settings;
  const { view: activeView } = data;

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
          const usernames = await publicClient.forgotUsername({
            keyHash: signingSession.keyHash,
          });

          setData({ usernames, connectorId, signingSession });
        } else {
          const keyHash = await connector.getKeyHash();
          const usernames = await publicClient.forgotUsername({ keyHash });
          setData({ usernames, connectorId, keyHash });
        }
        nextStep();
      } catch (err) {
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
        console.log(err);
      }
    },
  });

  const emailCredential = (
    <EmailCredential
      onAuth={() => signInWithCredential("privy")}
      goBack={reset}
      disableSignup
      email={data.email}
      setEmail={(email) => {
        setData({ email, view: "email" });
      }}
    />
  );

  const walletsCredential = (
    <div className="flex flex-col gap-7 w-full items-center">
      <div className="flex flex-col gap-4 w-full items-center">
        <AuthOptions action={signInWithCredential} isPending={isPending} />
        <Button size="sm" variant="link" onClick={reset}>
          <IconLeft className="w-[22px] h-[22px]" />
          <span>{m["common.back"]()}</span>
        </Button>
      </div>
    </div>
  );

  if (activeView === "wallets") return walletsCredential;
  if (activeView === "email") return emailCredential;

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-2 w-full">
      {emailCredential}
      <div className="w-full flex items-center justify-center gap-3">
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
        <p className="min-w-fit text-ink-placeholder-400 uppercase">{m["common.or"]()}</p>
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
      </div>
      <div className="flex flex-col items-center w-full gap-4">
        <SocialCredential onAuth={() => signInWithCredential("privy")} />
        <PasskeyCredential
          onAuth={() => signInWithCredential("passkey")}
          label={m["common.signWithPasskey"]({ action: "signin" })}
        />
        {isMd ? (
          <Button variant="secondary" fullWidth onClick={() => setData({ view: "wallets" })}>
            <IconWallet />
            {m["signin.connectWallet"]()}
          </Button>
        ) : null}
        {isMd ? null : (
          <Button
            fullWidth
            className="gap-2"
            variant="secondary"
            onClick={() => showModal(Modals.SignWithDesktop, { navigate })}
          >
            <IconQR className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
          </Button>
        )}
      </div>
      <div className="flex flex-col">
        <div className="flex justify-center items-center">
          <p>{m["signin.noAccount"]()}</p>
          <Button variant="link" onClick={() => navigate({ to: "/signup" })}>
            {m["common.signup"]()}
          </Button>
        </div>
        <div className="flex justify-center items-center text-center">
          <Button size="sm" variant="link" onClick={() => navigate({ to: "/" })}>
            <IconLeft className="w-[22px] h-[22px]" />
            <span>{m["common.back"]()}</span>
          </Button>
        </div>
      </div>
    </div>
  );
};

const UsernameStep: React.FC = () => {
  const { data, goToStep, reset } = useWizard<{
    usernames: Username[];
    keyHash?: Hex;
    signingSession?: SigningSession;
    connectorId: string;
  }>();

  const { toast } = useApp();
  const navigate = useNavigate();
  const { usernames, connectorId, keyHash, signingSession } = data;

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    session: signingSession,
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        }),
    },
    mutation: {
      onSuccess: () => navigate({ to: "/" }),
    },
  });

  const existUsernames = usernames.length;

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      {existUsernames ? (
        <div className="flex flex-col gap-4 w-full items-center">
          <UsernamesList
            usernames={usernames}
            onUserSelection={(username) => connectWithConnector({ username, connectorId, keyHash })}
          />
          <Button
            variant="link"
            onClick={() => {
              goToStep(0);
              reset();
            }}
            isLoading={isPending}
          >
            <IconLeft className="w-[22px] h-[22px]" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      ) : (
        <Button
          variant="link"
          onClick={() => {
            goToStep(0);
            reset();
          }}
        >
          <IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      )}
    </div>
  );
};

export const Signin = Object.assign(Container, {
  Credential: CredentialStep,
  Username: UsernameStep,
});
