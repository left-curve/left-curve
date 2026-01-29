import { useControlledState, useInputs } from "@left-curve/foundation";
import { useEffect, useState } from "react";
import { ActivityIndicator, View } from "react-native";
import { useConnectors } from "@left-curve/store";
import { useMutation, useQuery } from "@tanstack/react-query";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { wait } from "@left-curve/dango/utils";

import type React from "react";
import type { Connector } from "@left-curve/store/types";
import type Privy from "@privy-io/js-sdk-core";
import { PRIVY_ERRORS_MAPPING } from "~/constants";
import { Button, GlobalText, IconEmail, IconLeft, Input, OtpInput } from "../foundation";

type StepInputEmailProps = {
  value?: string;
  onChange: (email: string) => void;
  defaultValue?: string;
  onSubmit: () => void;
};

const StepInputEmail: React.FC<StepInputEmailProps> = ({
  value,
  defaultValue,
  onChange,
  onSubmit,
}) => {
  const connectors = useConnectors();
  const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };
  const [email, setEmail] = useControlledState(value, onChange, defaultValue);

  const { mutate, isPending } = useMutation({
    mutationFn: async () => {
      await connector.privy.auth.email.sendCode(email);
      setEmail(email);
    },
    onSuccess: () => onSubmit(),
  });

  return (
    <View className="w-full">
      <Input
        value={email}
        onChangeText={(text) => setEmail(text)}
        keyboardType="email-address"
        autoCapitalize="none"
        autoComplete="email"
        placeholder={m["auth.enterYourEmail"]()}
        startContent={<IconEmail className="text-ink-secondary-700" />}
        endContent={
          <Button variant="link" isLoading={isPending} onPress={() => mutate()}>
            <GlobalText>{m["common.submit"]()}</GlobalText>
          </Button>
        }
      />
    </View>
  );
};

type StepInputOptProps = {
  email: string;
  onSuccess: () => Promise<void>;
  disableSignup?: boolean;
  goBack: () => void;
};

const StepInputOtp: React.FC<StepInputOptProps> = ({ email, disableSignup, goBack, onSuccess }) => {
  const connectors = useConnectors();
  const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };
  const { setError, inputs, setValue } = useInputs();
  const [cooldown, setCooldown] = useState<number>(0);

  const otpValue = inputs.otp?.value || "";

  useEffect(() => {
    if (cooldown <= 0) return;
    const id = setInterval(() => {
      setCooldown((s) => (s > 0 ? s - 1 : 0));
    }, 1000);
    return () => clearInterval(id);
  }, [cooldown]);

  const { isLoading } = useQuery({
    enabled: otpValue.length === 6,
    queryKey: ["send-email-code", email, otpValue],
    queryFn: async () => {
      try {
        await connector.privy.auth.email.loginWithCode(
          email,
          otpValue,
          disableSignup ? "no-signup" : "login-or-sign-up",
          {
            embedded: {
              ethereum: {
                createOnLogin: "users-without-wallets",
              },
            },
          },
        );
        await wait(500);
        await onSuccess();
      } catch (e) {
        const message = "message" in (e as object) ? (e as Error).message : "authFailed";
        const error =
          PRIVY_ERRORS_MAPPING[message as keyof typeof PRIVY_ERRORS_MAPPING] ||
          m["auth.errors.authFailed"]();
        setError("otp", error);
      }
      return null;
    },
  });

  const label =
    cooldown > 0 ? `Resend in 00:${String(cooldown).padStart(2, "0")}` : "Click to resend";

  const handleResend = async () => {
    if (cooldown > 0) return;
    await connector.privy.auth.email.sendCode(email);
    setCooldown(60);
  };

  return (
    <View className="flex flex-col gap-6 w-full items-center text-center">
      <OtpInput
        length={6}
        value={otpValue}
        onChange={(code) => setValue("otp", code as string)}
        disabled={isLoading}
        errorMessage={inputs.otp?.error}
      />
      {isLoading && <ActivityIndicator size="small" color="#4D7FFF" />}
      <View className="flex flex-row justify-center items-center gap-2">
        <GlobalText>{m["auth.didntReceiveCode"]()}</GlobalText>
        <Button
          variant="link"
          classNames={{ base: "py-0 pl-0 h-fit" }}
          onPress={handleResend}
          isDisabled={cooldown > 0 || isLoading}
        >
          <GlobalText className="tabular-nums lining-nums">{label}</GlobalText>
        </Button>
      </View>
      <Button
        variant="link"
        onPress={goBack}
        leftIcon={<IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />}
      >
        <GlobalText className="leading-none pt-[2px]">{m["common.back"]()}</GlobalText>
      </Button>
    </View>
  );
};

export const EmailCredential = {
  Email: StepInputEmail,
  OTP: StepInputOtp,
};
