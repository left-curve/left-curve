import {
  Checkbox,
  createContext,
  ensureErrorMessage,
  ExpandOptions,
  IconWallet,
  useApp,
} from "@left-curve/applets-kit";
import { useSignupState } from "@left-curve/store";

import { Button, IconLeft, ResizerContainer } from "@left-curve/applets-kit";
import { AuthOptions } from "./AuthOptions";
import { EmailCredential } from "./EmailCredential";
import { SocialCredential } from "./SocialCredential";
import { PasskeyCredential } from "./PasskeyCredential";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";

import type React from "react";

const [SignupProvider, useSignup] = createContext<ReturnType<typeof useSignupState>>({
  name: "SignupContext",
});

type SignupProps = {
  goTo: (auth: "signin") => void;
  onFinish: () => void;
};

export const Signup: React.FC<SignupProps> = ({ goTo, onFinish }) => {
  const { toast } = useApp();

  const state = useSignupState({
    expiration: DEFAULT_SESSION_EXPIRATION,
    login: {
      onSuccess: () => onFinish(),
    },
    register: {
      onError: (e) =>
        toast.error({ title: m["errors.failureRequest"](), description: ensureErrorMessage(e) }),
    },
  });

  return (
    <SignupProvider value={state}>
      <ResizerContainer layoutId="signup" className="w-full max-w-[24.5rem]">
        <div className="flex items-center justify-center gap-8 px-4 flex-col w-full">
          <div className="flex flex-col gap-7 items-center justify-center w-full">
            <img
              src="./favicon.svg"
              alt="dango-logo"
              className="h-12 rounded-full shadow-account-card"
            />
            <div className="flex flex-col gap-3 items-center justify-center text-center">
              <h1 className="h2-heavy">{m["signup.title"]()}</h1>
              <Header />
            </div>
          </div>
          <Email />
          <Wallets />
          <Credentials />
          <Login />
          <Footer goTo={goTo} />
        </div>
      </ResizerContainer>
    </SignupProvider>
  );
};

const Header: React.FC = () => {
  const { screen, email } = useSignup();

  if (screen === "wallets") {
    return (
      <p className="text-ink-tertiary-500 diatype-m-medium">
        {m["signin.connectWalletToContinue"]()}
      </p>
    );
  }
  if (screen === "email") {
    return (
      <p className="text-ink-tertiary-500">
        {m["signin.sentVerificationCode"]()}
        <span className="font-bold ml-1">{email}</span>
      </p>
    );
  }

  if (screen === "login") {
    return (
      <p className="text-ink-tertiary-500 diatype-m-medium">
        {m["signup.accountCreated.description"]()}
      </p>
    );
  }

  return (
    <p className="text-ink-tertiary-500 diatype-m-medium">{m["signup.options.description"]()}</p>
  );
};

const Email: React.FC = () => {
  const { screen, setScreen, email, setEmail, register } = useSignup();

  if (screen !== "email") return null;

  return (
    <EmailCredential.OTP
      email={email}
      onSuccess={() => register.mutateAsync("privy")}
      goBack={() => {
        setScreen("options");
        setEmail("");
      }}
    />
  );
};

const Wallets: React.FC = () => {
  const { screen, setScreen, register } = useSignup();

  if (screen !== "wallets") return null;

  return (
    <div className="flex flex-col gap-7 w-full items-center">
      <div className="flex flex-col gap-4 w-full items-center">
        <AuthOptions action={register.mutateAsync} isPending={register.isPending} />
        <Button size="sm" variant="link" onClick={() => setScreen("options")}>
          <IconLeft className="w-[22px] h-[22px]" />
          <span>{m["common.back"]()}</span>
        </Button>
      </div>
    </div>
  );
};

const Credentials: React.FC = () => {
  const { screen, setScreen, register, email, setEmail } = useSignup();

  if (screen !== "options") return null;

  return (
    <div className="flex flex-col gap-6 w-full">
      <EmailCredential.Email value={email} onChange={setEmail} />

      <div className="w-full flex items-center justify-center gap-3">
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
        <p className="min-w-fit text-ink-placeholder-400 uppercase">{m["common.or"]()}</p>
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <SocialCredential onAuth={() => register.mutateAsync("privy")} signup />
        <PasskeyCredential
          onAuth={() => register.mutateAsync("passkey")}
          label={m["common.signWithPasskey"]({ action: "signup" })}
        />

        <Button variant="secondary" fullWidth onClick={() => setScreen("wallets")}>
          <IconWallet />
          {m["signin.connectWallet"]()}
        </Button>
      </div>
    </div>
  );
};

const Login: React.FC = () => {
  const { login, screen } = useSignup();
  const { settings, changeSettings } = useApp();
  const { useSessionKey } = settings;

  if (screen !== "login") return null;

  return (
    <div className="flex flex-col gap-6 w-full">
      <Button
        fullWidth
        onClick={() => login.mutateAsync({ useSessionKey })}
        isLoading={login.isPending}
      >
        {m["common.signin"]()}
      </Button>
      <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
        <div className="flex items-center gap-2 flex-col">
          <Checkbox
            size="md"
            label={m["common.signinWithSession"]()}
            checked={useSessionKey}
            onChange={(v) => changeSettings({ useSessionKey: v })}
          />
        </div>
      </ExpandOptions>
    </div>
  );
};

type FooterProps = {
  goTo: (auth: "signin") => void;
};

const Footer: React.FC<FooterProps> = ({ goTo }) => {
  const { screen } = useSignup();

  if (screen !== "options") return null;

  return (
    <div className="w-full flex flex-col items-center gap-1">
      <div className="flex items-center gap-1">
        <p>{m["signup.alreadyHaveAccount"]()}</p>
        <Button variant="link" autoFocus={false} onClick={() => goTo("signin")}>
          {m["common.signin"]()}
        </Button>
      </div>
    </div>
  );
};
