import { useInputs } from "@left-curve/foundation";
import { useEffect, useState } from "react";
import { useConnectors } from "@left-curve/store";
import { useMutation } from "@tanstack/react-query";

import { Button, IconEmail, IconLeft, Input, OtpInput } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { wait } from "@left-curve/dango/utils";
import { PRIVY_ERRORS_MAPPING } from "~/constants";

import type { Connector } from "@left-curve/store/types";
import type Privy from "@privy-io/js-sdk-core";

type EmailCredentialProps = {
  onAuth: () => void;
  disableSignup?: boolean;
  goBack: () => void;
  email: string;
  setEmail: (email?: string) => void;
};

export const EmailCredential: React.FC<EmailCredentialProps> = ({
  onAuth,
  goBack,
  disableSignup,
  email,
  setEmail,
}) => {
  const connectors = useConnectors();
  const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };

  if (!connector) return null;

  if (!email) {
    return (
      <StepInputEmail
        disableSignup={Boolean(disableSignup)}
        setEmail={setEmail}
        privy={connector.privy}
      />
    );
  }

  return (
    <StepInputOtp
      disableSignup={Boolean(disableSignup)}
      goBack={goBack}
      email={email}
      onAuth={onAuth}
      privy={connector.privy}
    />
  );
};

type StepInputEmailProps = {
  disableSignup: boolean;
  setEmail: (email: string) => void;
  privy: Privy;
};

const StepInputEmail: React.FC<StepInputEmailProps> = ({ privy, setEmail }) => {
  const { register, inputs } = useInputs();

  const { mutate, isPending } = useMutation({
    mutationFn: async () => {
      const email = inputs.email.value;
      await privy.auth.email.sendCode(email);
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
        {...register("email")}
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
  onAuth: () => void;
  disableSignup: boolean;
  goBack: () => void;
  privy: Privy;
};

const StepInputOtp: React.FC<StepInputOptProps> = ({
  email,
  disableSignup,
  privy,
  goBack,
  onAuth,
}) => {
  const { register, setError, inputs } = useInputs();
  const [cooldown, setCooldown] = useState<number>(0);

  useEffect(() => {
    if (cooldown <= 0) return;
    const id = setInterval(() => {
      setCooldown((s) => (s > 0 ? s - 1 : 0));
    }, 1000);
    return () => clearInterval(id);
  }, [cooldown]);

  useEffect(() => {
    if (inputs.otp?.value.length !== 6) return;
    (async () => {
      try {
        await privy.auth.email.loginWithCode(
          email,
          inputs.otp.value,
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
        onAuth();
      } catch (e) {
        const message = "message" in (e as object) ? (e as Error).message : "authFailed";
        const error = PRIVY_ERRORS_MAPPING[message as keyof typeof PRIVY_ERRORS_MAPPING];
        setError("otp", error);
      }
    })();
  }, [inputs.otp?.value]);

  const label =
    cooldown > 0 ? `Resend in 00:${String(cooldown).padStart(2, "0")}` : "Click to resend";

  const handleResend = async () => {
    if (cooldown > 0) return;
    await privy.auth.email.sendCode(email);
    setCooldown(60);
  };

  return (
    <div className="flex flex-col gap-6 w-full items-center text-center">
      <OtpInput length={6} {...register("otp")} />
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
