import { useAuthState, useConnectors } from "@left-curve/store";
import type { AuthScreen } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";

import {
  Button,
  Checkbox,
  createContext,
  ExpandOptions,
  IconKey,
  IconLeft,
  IconQR,
  IconWallet,
  Input,
  Modals,
  ResizerContainer,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { AuthOptions } from "./AuthOptions";
import { EmailCredential } from "./EmailCredential";
import { SocialCredential } from "./SocialCredential";
import { UsernamesList } from "./UsernamesList";
import { DangoLogo } from "../foundation/DangoLogo";

import type React from "react";

const [AuthProvider, useAuth] = createContext<ReturnType<typeof useAuthState>>({
  name: "AuthContext",
});

type AuthFlowProps = {
  onFinish: () => void;
  referrer?: number;
};

const getReferrerFromQuery = (): number | undefined => {
  if (typeof window === "undefined") return undefined;
  const ref = new URLSearchParams(window.location.search).get("ref");
  if (!ref) return undefined;
  const parsed = Number.parseInt(ref, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
};

export const AuthFlow: React.FC<AuthFlowProps> = ({ onFinish, referrer }) => {
  const { settings, toast } = useApp();

  const state = useAuthState({
    expiration: DEFAULT_SESSION_EXPIRATION,
    session: settings.useSessionKey,
    referrer: referrer ?? getReferrerFromQuery(),
    onSuccess: () => onFinish(),
    onError: (e) =>
      toast.error({
        title: m["common.error"](),
        description: e instanceof Error ? e.message : m["auth.errors.authFailed"](),
      }),
  });

  return (
    <AuthProvider value={state}>
      <ResizerContainer layoutId="auth" className="w-full">
        <div className="flex flex-col gap-7 items-center justify-center w-full">
          <ScreenRouter />
        </div>
      </ResizerContainer>
    </AuthProvider>
  );
};

const ScreenRouter: React.FC = () => {
  const { screen } = useAuth();

  const screens: Record<AuthScreen, React.FC> = {
    options: WelcomeScreen,
    email: EmailScreen,
    wallets: WalletsScreen,
    "passkey-choice": PasskeyChoiceScreen,
    "passkey-error": PasskeyErrorScreen,
    "create-account": CreateAccountScreen,
    "account-picker": AccountPickerScreen,
  };

  const Screen = screens[screen];
  return <Screen />;
};

const WelcomeScreen: React.FC = () => {
  const { authenticate, setScreen, setEmail, email } = useAuth();
  const { isMd } = useMediaQuery();
  const { showModal, settings, changeSettings } = useApp();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <h1 className="h2-heavy">{m["common.welcomeToDango"]()}</h1>
      </div>

      <div className="flex items-center justify-center flex-col gap-8 w-full">
        <EmailCredential.Email
          value={email}
          onChange={setEmail}
          onSubmit={() => setScreen("email")}
        />

        <div className="w-full flex items-center justify-center gap-3">
          <span className="h-[1px] bg-outline-secondary-gray flex-1" />
          <p className="min-w-fit text-ink-placeholder-400 uppercase">{m["common.or"]()}</p>
          <span className="h-[1px] bg-outline-secondary-gray flex-1" />
        </div>

        <div className="flex flex-col items-center w-full gap-4">
          <SocialCredential onAuth={() => authenticate.mutateAsync("privy")} />

          <Button
            fullWidth
            variant="secondary"
            className="gap-2"
            onClick={() => authenticate.mutateAsync("passkey")}
          >
            <IconKey className="w-6 h-6" />
            <p>{m["common.connectWithPasskey"]()}</p>
          </Button>

          <Button variant="secondary" fullWidth onClick={() => setScreen("wallets")}>
            <IconWallet />
            {m["common.connectWallet"]()}
          </Button>

          {!isMd && (
            <Button
              fullWidth
              className="gap-2"
              variant="secondary"
              onClick={() => showModal(Modals.SignWithDesktop)}
            >
              <IconQR className="w-6 h-6" />
              <p className="min-w-20">{m["common.signinWithDesktop"]()}</p>
            </Button>
          )}
        </div>
      </div>

      <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
        <div className="flex items-center gap-2 flex-col">
          <Checkbox
            size="md"
            label={m["common.signinWithSession"]()}
            checked={settings.useSessionKey}
            onChange={(v) => changeSettings({ useSessionKey: v })}
          />
        </div>
      </ExpandOptions>
    </>
  );
};

const EmailScreen: React.FC = () => {
  const { email, setEmail, setScreen, authenticate } = useAuth();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["common.welcomeToDango"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["signin.sentVerificationCode"]()}
            <span className="font-bold ml-1">{email}</span>
          </p>
        </div>
      </div>

      <EmailCredential.OTP
        email={email}
        onSuccess={() => authenticate.mutateAsync("privy")}
        goBack={() => {
          setScreen("options");
          setEmail("");
        }}
      />
    </>
  );
};

const WalletsScreen: React.FC = () => {
  const { setScreen, authenticate } = useAuth();
  const connectors = useConnectors();
  const hasWallets =
    connectors.filter((c) => !["passkey", "session", "privy"].includes(c.type)).length > 0;

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["common.welcomeToDango"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {hasWallets ? m["signin.connectWalletToContinue"]() : m["common.notWalletDetected"]()}
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-7 w-full items-center">
        <div className="flex flex-col gap-4 w-full items-center">
          <AuthOptions
            action={(id) => authenticate.mutateAsync(id)}
            isPending={authenticate.isPending}
          />
          <Button size="sm" variant="link" onClick={() => setScreen("options")}>
            <IconLeft className="w-[22px] h-[22px]" />
            <span>{m["common.back"]()}</span>
          </Button>
        </div>
      </div>
    </>
  );
};

const PasskeyChoiceScreen: React.FC = () => {
  const { setScreen, passkeyCreate, passkeyLogin } = useAuth();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["auth.passkeyChoice.title"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["auth.passkeyChoice.description"]()}
          </p>
        </div>
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <Button
          fullWidth
          variant="secondary"
          onClick={() => passkeyCreate.mutateAsync()}
          isLoading={passkeyCreate.isPending}
        >
          {m["auth.passkeyChoice.createNew"]()}
        </Button>

        <Button
          fullWidth
          variant="secondary"
          onClick={() => passkeyLogin.mutateAsync()}
          isLoading={passkeyLogin.isPending}
        >
          {m["auth.passkeyChoice.useExisting"]()}
        </Button>

        <Button variant="link" onClick={() => setScreen("options")}>
          <IconLeft className="w-[22px] h-[22px]" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      </div>
    </>
  );
};

const PasskeyErrorScreen: React.FC = () => {
  const { passkeyCreate, setScreen } = useAuth();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["auth.passkeyError.title"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["auth.passkeyError.description"]()}
          </p>
        </div>
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <Button
          fullWidth
          onClick={() => passkeyCreate.mutateAsync()}
          isLoading={passkeyCreate.isPending}
        >
          {m["signup.createAccount"]()}
        </Button>

        <Button variant="link" onClick={() => setScreen("passkey-choice")}>
          <IconLeft className="w-[22px] h-[22px]" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      </div>
    </>
  );
};

const truncateIdentifier = (id: string) => {
  if (id.length > 20) return `${id.slice(0, 6)}...${id.slice(-4)}`;
  return id;
};

const CreateAccountScreen: React.FC = () => {
  const { createAccount, identifier, referrer, setReferrer, setScreen } = useAuth();
  const { settings, changeSettings } = useApp();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["auth.createYourAccount"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["auth.noAccountFound"]({ identifier: truncateIdentifier(identifier || "") })}
          </p>
        </div>
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <Button
          fullWidth
          onClick={() => createAccount.mutateAsync()}
          isLoading={createAccount.isPending}
        >
          {m["common.continue"]()}
        </Button>
        <Input
          fullWidth
          placeholder={m["auth.referralCode"]()}
          value={referrer?.toString() ?? ""}
          onChange={(e) => {
            const val = e.target.value;
            setReferrer(val ? Number.parseInt(val, 10) || undefined : undefined);
          }}
        />

        <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
          <div className="flex items-center gap-2 flex-col w-full">
            <Checkbox
              size="md"
              label={m["common.signinWithSession"]()}
              checked={settings.useSessionKey}
              onChange={(v) => changeSettings({ useSessionKey: v })}
            />
          </div>
        </ExpandOptions>

        <Button variant="link" onClick={() => setScreen("options")}>
          <IconLeft className="w-[22px] h-[22px]" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      </div>
    </>
  );
};

const AccountPickerScreen: React.FC = () => {
  const { users, selectAccount, createNewWithExistingKey, setScreen } = useAuth();

  return (
    <>
      <div className="flex flex-col gap-7 items-center justify-center w-full text-center">
        <DangoLogo className="h-[60px]" />
        <div className="flex flex-col gap-3">
          <h1 className="h2-heavy">{m["signin.usernamesFound"]()}</h1>
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["signin.chooseCredential"]()}</p>
        </div>
      </div>

      <div className="flex flex-col gap-4 w-full items-center">
        <UsernamesList
          users={users}
          onUserSelection={(userIndex) => selectAccount.mutateAsync(userIndex)}
        />

        <Button
          fullWidth
          variant="secondary"
          onClick={() => createNewWithExistingKey.mutateAsync()}
          isLoading={createNewWithExistingKey.isPending}
        >
          {m["common.createNewUser"]()}
        </Button>

        <Button
          variant="link"
          onClick={() => setScreen("options")}
          isLoading={selectAccount.isPending}
        >
          <IconLeft className="w-[22px] h-[22px]" />
          <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
        </Button>
      </div>
    </>
  );
};
