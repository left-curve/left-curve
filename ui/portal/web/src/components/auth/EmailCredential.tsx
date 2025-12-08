import { useControlledState, useInputs } from "@left-curve/foundation";
import { useEffect, useState } from "react";
import { useConnectors } from "@left-curve/store";
import { useMutation, useQuery } from "@tanstack/react-query";

import { Button, IconEmail, IconLeft, Input, OtpInput, Spinner } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { wait } from "@left-curve/dango/utils";
import { PRIVY_ERRORS_MAPPING } from "~/constants";

import type { Connector } from "@left-curve/store/types";
import type Privy from "@privy-io/js-sdk-core";

type StepInputEmailProps = {
  value?: string;
  onChange: (email: string) => void;
  defaultValue?: string;
};

const StepInputEmail: React.FC<StepInputEmailProps> = ({ value, defaultValue, onChange }) => {
  const connectors = useConnectors();
  const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };
  const [email, setEmail] = useControlledState(value, onChange, defaultValue);

  const { mutate, isPending } = useMutation({
    mutationFn: async () => {
      await connector.privy.auth.email.sendCode(email);
      setEmail(email);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        mutate();
      }}
      className="w-full"
    >
      <Input
        fullWidth
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        startContent={<IconEmail />}
        endContent={
          <Button variant="link" className="p-0" isLoading={isPending} type="submit">
            {m["common.submit"]()}
          </Button>
        }
        placeholder={
          <span>
            {m["auth.enterYou"]()}{" "}
            <span className="exposure-m-italic text-ink-secondary-rice">email</span>
          </span>
        }
      />
    </form>
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
  const { register, setError, inputs } = useInputs();
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
    <div className="flex flex-col gap-6 w-full items-center text-center">
      <OtpInput length={6} {...register("otp")} />
      {isLoading && <Spinner size="sm" color="blue" />}
      <div className="flex justify-center items-center gap-2">
        <p>{m["auth.didntReceiveCode"]()}</p>
        <Button
          variant="link"
          className="py-0 pl-0 h-fit tabular-nums lining-nums"
          onClick={handleResend}
          isDisabled={cooldown > 0}
        >
          {label}
        </Button>
      </div>
      <Button variant="link" onClick={goBack}>
        <IconLeft className="w-[22px] h-[22px] text-primitives-blue-light-500" />
        <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
      </Button>
    </div>
  );
};

export const EmailCredential = {
  Email: StepInputEmail,
  OTP: StepInputOtp,
};
