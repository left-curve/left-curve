import { Button, IconEmail, IconLeft, Input, OtpInput } from "@left-curve/applets-kit";
import { useInputs } from "@left-curve/foundation";
import { useLoginWithEmail, usePrivy } from "@privy-io/react-auth";
import { useEffect, useState } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useMutation } from "@tanstack/react-query";
import { wait } from "@left-curve/dango/utils";

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
  if (!email) {
    return <StepInputEmail disableSignup={Boolean(disableSignup)} setEmail={setEmail} />;
  }

  return (
    <StepInputOtp
      disableSignup={Boolean(disableSignup)}
      goBack={goBack}
      email={email}
      onAuth={onAuth}
    />
  );
};

type StepInputEmailProps = {
  disableSignup: boolean;
  setEmail: (email: string) => void;
};

const StepInputEmail: React.FC<StepInputEmailProps> = ({ disableSignup, setEmail }) => {
  const { register, inputs } = useInputs();
  const { sendCode } = useLoginWithEmail();

  const { mutate, isPending } = useMutation({
    mutationFn: async () => {
      const email = inputs.email.value;
      sendCode({ email, disableSignup });
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
};

const StepInputOtp: React.FC<StepInputOptProps> = ({ email, disableSignup, goBack, onAuth }) => {
  const { register, setError, inputs } = useInputs();
  const { sendCode, loginWithCode } = useLoginWithEmail();
  const { createWallet } = usePrivy();
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
        await loginWithCode({ code: inputs.otp.value });
        if (!disableSignup) await createWallet();
        await wait(500);
        onAuth();
      } catch (e) {
        const message = "message" in (e as object) ? (e as Error).message : "something wen't wrong";
        setError("otp", message);
      }
    })();
  }, [inputs.otp?.value]);

  const label =
    cooldown > 0 ? `Resend in 00:${String(cooldown).padStart(2, "0")}` : "Click to resend";

  const handleResend = async () => {
    if (cooldown > 0) return;
    await sendCode({ email });
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
