import { useSigninState } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";

import {
  Button,
  createContext,
  IconLeft,
  IconQR,
  IconWallet,
  Modals,
  ResizerContainer,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { AuthOptions } from "./AuthOptions";
import { EmailCredential } from "./EmailCredential";
import { SocialCredential } from "./SocialCredential";
import { PasskeyCredential } from "./PasskeyCredential";
import { UsernamesList } from "./UsernamesList";

import type React from "react";

const [SigninProvider, useSignin] = createContext<ReturnType<typeof useSigninState>>({
  name: "SigninContext",
});

type SignInProps = {
  goTo: (auth: "signup") => void;
};

export const Signin: React.FC<SignInProps> = ({ goTo }) => {
  const { settings, toast } = useApp();

  const state = useSigninState({
    expiration: DEFAULT_SESSION_EXPIRATION,
    session: settings.useSessionKey,
    connect: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        }),
    },
  });

  return (
    <SigninProvider value={state}>
      <ResizerContainer layoutId="signin" className="w-full max-w-[24.5rem]">
        <div className="flex flex-col gap-7 items-center justify-center w-full px-4">
          <Header />
          <Email />
          <Wallets />
          <Credentials />
          <UsernamesSection />
          <Footer goTo={goTo} />
        </div>
      </ResizerContainer>
    </SigninProvider>
  );
};

const Header: React.FC = () => {
  const { screen, email, usernames } = useSignin();

  let title = m["common.signin"]();
  let description: React.ReactNode = null;

  if (screen === "usernames") {
    if (usernames.length > 0) {
      title = m["signin.usernamesFound"]();
      description = m["signin.chooseCredential"]();
    } else {
      title = m["signin.noUsernamesFound"]();
      description = m["signin.noUsernameMessage"]();
    }
  } else if (screen === "wallets") {
    description = m["signin.connectWalletToContinue"]();
  } else if (screen === "email") {
    description = (
      <span className="text-ink-tertiary-500">
        {m["signin.sentVerificationCode"]()}
        <span className="font-bold ml-1">{email}</span>
      </span>
    );
  }

  return (
    <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
      <img src="./favicon.svg" alt="dango-logo" className="h-12 rounded-full shadow-account-card" />
      <div className="flex flex-col gap-3">
        <h1 className="h2-heavy">{title}</h1>
        {description && <div className="text-ink-tertiary-500 diatype-m-medium">{description}</div>}
      </div>
    </div>
  );
};

const Email: React.FC = () => {
  const { screen, setScreen, email, setEmail, connect } = useSignin();

  if (screen !== "email") return null;

  return (
    <EmailCredential.OTP
      email={email}
      disableSignup
      onSuccess={() => connect.mutateAsync("privy")}
      goBack={() => {
        setScreen("options");
        setEmail("");
      }}
    />
  );
};

const Wallets: React.FC = () => {
  const { screen, setScreen, connect } = useSignin();

  if (screen !== "wallets") return null;

  return (
    <div className="flex flex-col gap-7 w-full items-center">
      <div className="flex flex-col gap-4 w-full items-center">
        <AuthOptions action={(id) => connect.mutateAsync(id)} isPending={connect.isPending} />
        <Button size="sm" variant="link" onClick={() => setScreen("options")}>
          <IconLeft className="w-[22px] h-[22px]" />
          <span>{m["common.back"]()}</span>
        </Button>
      </div>
    </div>
  );
};

const Credentials: React.FC = () => {
  const { screen, setScreen, connect, setEmail, email } = useSignin();
  const { isMd } = useMediaQuery();
  const { showModal } = useApp();

  if (screen !== "options") return null;

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-2 w-full">
      <EmailCredential.Email value={email} onChange={setEmail} />

      <div className="w-full flex items-center justify-center gap-3">
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
        <p className="min-w-fit text-ink-placeholder-400 uppercase">{m["common.or"]()}</p>
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <SocialCredential onAuth={() => connect.mutateAsync("privy")} />
        <PasskeyCredential
          onAuth={() => connect.mutateAsync("passkey")}
          label={m["common.signWithPasskey"]({ action: "signin" })}
        />

        {isMd ? (
          <Button variant="secondary" fullWidth onClick={() => setScreen("wallets")}>
            <IconWallet />
            {m["signin.connectWallet"]()}
          </Button>
        ) : (
          <Button
            fullWidth
            className="gap-2"
            variant="secondary"
            onClick={() => showModal(Modals.SignWithDesktop)}
          >
            <IconQR className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
          </Button>
        )}
      </div>
    </div>
  );
};

const UsernamesSection: React.FC = () => {
  const { screen, setScreen, usernames, login } = useSignin();

  const goBack = () => setScreen("options");

  if (screen !== "usernames") return null;

  const existUsernames = usernames.length > 0;

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      {existUsernames ? (
        <div className="flex flex-col gap-4 w-full items-center">
          <UsernamesList
            usernames={usernames}
            onUserSelection={(username) => login.mutateAsync(username)}
          />
          <Button variant="link" onClick={goBack} isLoading={login.isPending}>
            <IconLeft className="w-[22px] h-[22px]" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      ) : (
        <Button variant="link" onClick={goBack}>
          <IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      )}
    </div>
  );
};

type FooterProps = {
  goTo: (auth: "signup") => void;
};

const Footer: React.FC<FooterProps> = ({ goTo }) => {
  const { screen } = useSignin();

  if (screen !== "options") return null;

  return (
    <div className="flex justify-center items-center">
      <p>{m["signin.noAccount"]()}</p>
      <Button variant="link" onClick={() => goTo("signup")}>
        {m["common.signup"]()}
      </Button>
    </div>
  );
};
