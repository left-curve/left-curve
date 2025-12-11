import { createContext, ensureErrorMessage } from "@left-curve/applets-kit";
import { useSignupState } from "@left-curve/store";
import { useApp } from "@left-curve/foundation";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import { View } from "react-native";
import {
  Button,
  GlobalText,
  IconGoogle,
  IconKey,
  IconTwitter,
  IconWallet,
  Input,
} from "../foundation";
import { useRouter } from "expo-router";

const [SignupProvider, useSignup] = createContext<ReturnType<typeof useSignupState>>({
  name: "SignupContext",
});

type SignupProps = {
  goTo: () => void;
};

export const Signup: React.FC<SignupProps> = ({ goTo }) => {
  const state = useSignupState({
    expiration: 24 * 60 * 60 * 1000, // 24 hours
    register: {
      onError: (e) => console.log("Toast error:", ensureErrorMessage(e)),
    },
  });

  return (
    <SignupProvider value={state}>
      <View className="flex items-center justify-center gap-8 flex-col w-full">
        <GlobalText>{m["signup.title"]()}</GlobalText>
        <Header />
      </View>
      <Email />
      <Wallets />
      <Credentials />
      <Login />
      <Deposit />
      <Footer goTo={goTo} />
    </SignupProvider>
  );
};

const Header: React.FC = () => {
  const { screen, email } = useSignup();

  if (screen === "wallets") {
    return (
      <GlobalText className="text-ink-tertiary-500 diatype-m-medium text-center">
        {m["signin.connectWalletToContinue"]()}
      </GlobalText>
    );
  }

  if (screen === "email") {
    return (
      <View className="flex flex-row gap-1 items-cente text-center">
        <GlobalText className="text-ink-tertiary-500">
          {m["signin.sentVerificationCode"]()}
        </GlobalText>
        <GlobalText className="font-bold ml-1 text-ink-tertiary-500">{email}</GlobalText>
      </View>
    );
  }

  if (screen === "login") {
    return (
      <GlobalText className="text-ink-tertiary-500 diatype-m-medium text-center">
        {m["signup.accountCreated.description"]()}
      </GlobalText>
    );
  }

  return (
    <GlobalText className="text-ink-tertiary-500 diatype-m-medium text-center">
      {m["signup.options.description"]()}
    </GlobalText>
  );
};

const Email: React.FC = () => {
  const { screen } = useSignup();

  if (screen !== "email") return null;

  return <GlobalText>OTP Input</GlobalText>;
};

const Wallets: React.FC = () => {
  const { screen } = useSignup();

  if (screen !== "wallets") return null;

  return <GlobalText>Wallet Connect Options</GlobalText>;
};

const Credentials: React.FC = () => {
  const { screen, setScreen, email, setEmail } = useSignup();

  if (screen !== "options") return null;

  return (
    <View className="flex flex-col gap-6 w-full">
      <Input
        className="border border-ink-secondary-500 rounded-md p-2 w-full"
        placeholder="Email"
      />

      <View className="w-full flex-row items-center justify-center gap-3">
        <View className="h-[1px] bg-outline-secondary-gray flex-1" />
        <GlobalText className="text-ink-placeholder-400 uppercase">{m["common.or"]()}</GlobalText>
        <View className="h-[1px] bg-outline-secondary-gray flex-1" />
      </View>

      <View className="flex flex-col items-center w-full gap-4">
        <View className="w-full flex-row gap-3 items-center">
          <View className="flex-1">
            <Button variant="secondary" classNames={{ base: "w-full" }}>
              <IconGoogle className="w-6 h-6" />
            </Button>
          </View>
          <View className="flex-1">
            <Button variant="secondary" classNames={{ base: "w-full" }}>
              <IconTwitter className="w-6 h-6" />
            </Button>
          </View>
        </View>
        <Button
          className="gap-2"
          variant="secondary"
          classNames={{ base: "w-full" }}
          leftIcon={<IconKey className="w-6 h-6" />}
        >
          <GlobalText>{m["common.signWithPasskey"]({ action: "signup" })}</GlobalText>
        </Button>

        <Button
          className="gap-2"
          variant="secondary"
          classNames={{ base: "w-full" }}
          leftIcon={<IconWallet className="w-6 h-6" />}
          onPress={() => setScreen("wallets")}
        >
          <GlobalText>{m["signin.connectWallet"]()}</GlobalText>
        </Button>
      </View>
    </View>
  );
};

const Login: React.FC = () => {
  const { login, screen } = useSignup();
  const { settings, changeSettings } = useApp();
  const { useSessionKey } = settings;

  if (screen !== "login") return null;

  return (
    <View className="flex flex-col gap-6 w-full">
      <Button onPress={() => login.mutateAsync({ useSessionKey })} isLoading={login.isPending}>
        {m["common.signin"]()}
      </Button>
      {/* <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
        <div className="flex items-center gap-2 flex-col">
          <Checkbox
            size="md"
            label={m["common.signinWithSession"]()}
            checked={useSessionKey}
            onChange={(v) => changeSettings({ useSessionKey: v })}
          />
        </div>
      </ExpandOptions> */}
    </View>
  );
};

const Deposit: React.FC = () => {
  const { screen } = useSignup();
  const { navigate } = useRouter();

  if (screen !== "deposit") return null;

  return (
    <div className="flex flex-col gap-6 w-full items-center">
      <div className="flex items-center flex-col gap-5">
        <img
          src="/images/account-creation/deposit.svg"
          alt="deposit-bag"
          className="w-[60px] h-[60px]"
        />
        <div className="flex flex-col w-full items-center gap-1">
          <h2 className="h4-bold text-ink-secondary-700">{m["signup.deposit.title"]()}</h2>
          <p className="diatype-m-regular">{m["signup.deposit.description"]()}</p>
          <p className="diatype-m-regular">{m["signup.deposit.description2"]()}</p>
        </div>
      </div>
      <Button className="min-w-[11.25rem]" onPress={() => navigate("/bridge")}>
        {m["signup.deposit.cta"]()}
      </Button>
    </div>
  );
};

type FooterProps = {
  goTo: () => void;
};

const Footer: React.FC<FooterProps> = ({ goTo }) => {
  const { screen } = useSignup();

  if (screen !== "options") return null;

  return (
    <View className="w-full flex flex-col items-center gap-1">
      <View className="flex flex-row items-center gap-1">
        <GlobalText>{m["signup.alreadyHaveAccount"]()}</GlobalText>
        <Button variant="link" onPress={goTo}>
          {m["common.signin"]()}
        </Button>
      </View>
    </View>
  );
};
