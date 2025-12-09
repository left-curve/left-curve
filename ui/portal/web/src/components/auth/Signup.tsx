import {
  Checkbox,
  createContext,
  ensureErrorMessage,
  ExpandOptions,
  IconDango,
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
    register: {
      onError: (e) =>
        toast.error({ title: m["errors.failureRequest"](), description: ensureErrorMessage(e) }),
    },
  });

  return (
    <SignupProvider value={state}>
      <ResizerContainer layoutId="signup" className="w-full">
        <div className="flex items-center justify-center gap-8 flex-col w-full">
          {state.screen !== "deposit" ? (
            <div className="flex flex-col gap-7 items-center justify-center w-full">
              <IconDango className="w-[60px] h-[60px]" />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">{m["signup.title"]()}</h1>
                <Header />
              </div>
            </div>
          ) : null}
          <Email />
          <Wallets />
          <Credentials />
          <Login />
          <Deposit onFinish={onFinish} />
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
      <EmailCredential.Email
        value={email}
        onChange={setEmail}
        onSubmit={() => setScreen("email")}
      />

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

const Deposit: React.FC<{ onFinish: () => void }> = ({ onFinish }) => {
  const { screen } = useSignup();
  const { navigate } = useApp();

  if (screen !== "deposit") return null;

  return (
    <div className="flex flex-col gap-6 w-full items-center">
      <div className="flex items-center flex-col gap-5">
        <img
          src="/images/account-creation/deposit.svg"
          alt="deposit-bag"
          className="w-[60px] h-[60px]"
        />
        <div className="flex flex-col w-full text.center items-center gap-1">
          <h2 className="h4-bold text-ink-secondary-700">{m["signup.deposit.title"]()}</h2>
          <p className="diatype-m-regular">{m["signup.deposit.description"]()}</p>
          <p className="diatype-m-regular">{m["signup.deposit.description2"]()}</p>
        </div>
      </div>
      <Button className="min-w-[11.25rem]" onClick={() => [navigate("/bridge"), onFinish()]}>
        {m["signup.deposit.cta"]()}
      </Button>
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
