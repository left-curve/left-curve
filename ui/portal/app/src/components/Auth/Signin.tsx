import { useSigninState } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useApp } from "@left-curve/foundation";

import type React from "react";
import { createContext } from "@left-curve/foundation";
import { View } from "react-native";
import {
  Button,
  GlobalText,
  IconGoogle,
  IconKey,
  IconLeft,
  IconQR,
  IconTwitter,
  Input,
} from "../foundation";

const [SigninProvider, useSignin] = createContext<ReturnType<typeof useSigninState>>({
  name: "SigninContext",
});

type SignInProps = {
  goTo: () => void;
};

export const Signin: React.FC<SignInProps> = ({ goTo }) => {
  const { settings } = useApp();

  const state = useSigninState({
    expiration: 24 * 60 * 60 * 1000, // 24 hours
    session: settings.useSessionKey,
    connect: {
      error: () => console.error("Error during signin"),
    },
  });

  return (
    <SigninProvider value={state}>
      <View className="flex flex-col gap-7 items-center justify-center w-full">
        <Header />
        <Email />
        <Wallets />
        <Credentials />
        <UsernamesSection />
        <Footer goTo={goTo} />
      </View>
    </SigninProvider>
  );
};

const Header: React.FC = () => {
  const { screen, email, usersIndexAndName } = useSignin();

  let title = m["common.signin"]();
  let description: React.ReactNode = null;

  if (screen === "usernames") {
    if (usersIndexAndName.length > 0) {
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
      <GlobalText className="text-ink-tertiary-500">
        {m["signin.sentVerificationCode"]()}
        <GlobalText className="font-bold ml-1 text-ink-tertiary-500">{email}</GlobalText>
      </GlobalText>
    );
  }

  return (
    <View className="flex flex-col gap-7 items-center justify-center w-full text-center">
      <View className="flex flex-col gap-3 items-center">
        <GlobalText className="h2-heavy">{title}</GlobalText>

        {description && (
          <GlobalText className="text-ink-tertiary-500 diatype-m-medium text-center">
            {description}
          </GlobalText>
        )}
      </View>
    </View>
  );
};

const Email: React.FC = () => {
  const { screen } = useSignin();

  if (screen !== "email") return null;

  return <GlobalText>OTP Input</GlobalText>;
};

const Wallets: React.FC = () => {
  const { screen } = useSignin();

  if (screen !== "wallets") return null;

  return <GlobalText>Wallet Connect Options</GlobalText>;
};

const Credentials: React.FC = () => {
  const { screen } = useSignin();

  if (screen !== "options") return null;

  return (
    <View className="flex items-center justify-center flex-col gap-8 px-2 w-full">
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
          <GlobalText>{m["common.signWithPasskey"]({ action: "signin" })}</GlobalText>
        </Button>

        <Button
          className="gap-2"
          variant="secondary"
          classNames={{ base: "w-full" }}
          leftIcon={<IconQR className="w-6 h-6" />}
        >
          <GlobalText> {m["common.signinWithDesktop"]()}</GlobalText>
        </Button>
      </View>
    </View>
  );
};

const UsernamesSection: React.FC = () => {
  const { screen, setScreen, usersIndexAndName, login } = useSignin();

  const goBack = () => setScreen("options");

  if (screen !== "usernames") return null;

  const existUsernames = usersIndexAndName.length > 0;

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      {existUsernames ? (
        <View className="flex flex-col gap-4 w-full items-center">
          <GlobalText>Usernames List</GlobalText>
          {/* <UsernamesList
            usernames={usernames}
            onUserSelection={(username) => login.mutateAsync(username)}
          /> */}
          <Button
            variant="link"
            onPress={goBack}
            isLoading={login.isPending}
            leftIcon={<IconLeft className="w-[22px] h-[22px]" />}
          >
            <GlobalText className="leading-none pt-[2px]">{m["common.back"]()}</GlobalText>
          </Button>
        </View>
      ) : (
        <Button
          variant="link"
          onPress={goBack}
          leftIcon={<IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />}
        >
          <GlobalText className="leading-none pt-[2px]">{m["common.back"]()}</GlobalText>
        </Button>
      )}
    </div>
  );
};

type FooterProps = {
  goTo: () => void;
};

const Footer: React.FC<FooterProps> = ({ goTo }) => {
  const { screen } = useSignin();

  if (screen !== "options") return null;

  return (
    <View className="w-full flex flex-col items-center gap-1">
      <View className="flex flex-row items-center gap-1">
        <GlobalText>{m["signin.noAccount"]()}</GlobalText>
        <Button variant="link" onPress={goTo}>
          {m["common.signup"]()}
        </Button>
      </View>
    </View>
  );
};
