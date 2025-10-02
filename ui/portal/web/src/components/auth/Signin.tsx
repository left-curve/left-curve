import {
  IconApple,
  IconEmail,
  IconGoogle,
  IconPasskey,
  IconWallet,
  Input,
  Modals,
  OtpInput,
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
import { Link } from "@tanstack/react-router";
import { AuthCarousel } from "./AuthCarousel";
import { UsernamesList } from "./UsernamesList";

import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

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
    <div className="h-svh xl:h-screen w-screen flex items-center justify-center bg-surface-primary-rice text-ink-primary-900">
      <div className="flex items-center justify-center flex-1 min-w-fit">
        <ResizerContainer layoutId="signin" className="w-full max-w-[24.5rem]">
          {children}
        </ResizerContainer>
      </div>
      <AuthCarousel />
    </div>
  );
};

const CredentialStep: React.FC = () => {
  const { toast, settings, showModal } = useApp();
  const { createSessionKey } = useSessionKey();
  const { nextStep, setData } = useWizard();
  const { useSessionKey: session } = settings;
  const connectors = useConnectors();
  const navigate = useNavigate();
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

  const onWalletSuccess = () => {
    nextStep();
  };

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-2">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-account-card"
        />
        <h1 className="h2-heavy">{m["common.signin"]()}</h1>
      </div>

      <Input
        fullWidth
        type="email"
        startContent={<IconEmail />}
        endContent={
          <Button variant="link" className="p-0">
            Submit
          </Button>
        }
        placeholder={
          <span>
            Enter your <span className="exposure-m-italic text-ink-secondary-rice">email</span>
          </span>
        }
      />

      <div className="w-full flex items-center justify-center gap-3">
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
        <p className="min-w-fit text-ink-placeholder-400">OR</p>
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        {/* {isMd ? (
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
            onClick={() => showModal(Modals.SignWithDesktop, { navigate })}
            className="gap-2"
            variant="secondary"
          >
            <IconQR className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
          </Button>
        )} */}
        <div className="grid grid-cols-2 gap-3 w-full">
          <Button onClick={nextStep} variant="secondary" fullWidth>
            <IconGoogle />
          </Button>
          <Button onClick={nextStep} variant="secondary" fullWidth>
            <IconApple />
          </Button>
        </div>
        <Button
          fullWidth
          onClick={() => signInWithCredential("passkey")}
          isLoading={isPending}
          className="gap-2"
          variant="secondary"
        >
          <IconPasskey className="w-6 h-6" />
          <p className="min-w-20"> {m["common.signWithPasskey"]({ action: "signin" })}</p>
        </Button>
        <Button
          fullWidth
          onClick={() => showModal(Modals.ConnectWallet, { onSuccess: onWalletSuccess })}
          isLoading={isPending}
          className="gap-2"
          variant="secondary"
        >
          <IconWallet className="w-6 h-6" />
          <p className="min-w-20">Connect wallet</p>
        </Button>
      </div>

      <div className="flex flex-col">
        <div className="flex justify-center items-center">
          <p>{m["signin.noAccount"]()}</p>
          <Button variant="link" onClick={() => navigate({ to: "/signup" })} isDisabled={isPending}>
            {m["common.signup"]()}
          </Button>
        </div>
        <Button as={Link} fullWidth variant="tertiary-red" to="/">
          {m["signin.continueWithoutSignin"]()}
        </Button>
      </div>
    </div>
  );
};

const VerifyEmailStep: React.FC = () => {
  const { previousStep } = useWizard<{
    usernames: Username[];
    keyHash?: Hex;
    signingSession?: SigningSession;
    connectorId: string;
  }>();

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-account-card"
        />
        <h1 className="h2-heavy">{m["common.signin"]()}</h1>
        <p className="text-ink-tertiary-500">
          We've sent a verification code to{" "}
          <span className="font-bold">phuongmai035@gmail.com</span>
        </p>
      </div>
      <OtpInput length={4} />
      <div className="flex justify-center items-center gap-2">
        <p>Didn't receive the code?</p>
        <Button variant="link" className="py-0 pl-0 h-fit">
          Click to resend
        </Button>
      </div>
      <Button fullWidth>Sign in</Button>
      <Button variant="link" onClick={previousStep}>
        <IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />
        <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
      </Button>
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
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-account-card"
        />
        {existUsernames ? (
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
      {existUsernames ? (
        <div className="flex flex-col gap-4 w-full items-center">
          <UsernamesList
            usernames={usernames}
            onUserSelection={(username) => connectWithConnector({ username, connectorId, keyHash })}
          />
          <Button variant="link" onClick={previousStep} isLoading={isPending}>
            <IconLeft className="w-[22px] h-[22px]" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      ) : (
        <Button variant="link" onClick={previousStep}>
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
  VerifyEmail: VerifyEmailStep,
});
