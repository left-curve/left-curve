import { DangoButton, Input, useWizard } from "@dango/shared";
import { usePublicClient } from "@leftcurve/react";
import type React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";

export const LoginStep: React.FC = () => {
  const { nextStep, setData, previousStep, data } = useWizard();
  const { setError, register, watch, setValue, formState } = useForm<{
    username: string;
    retry: boolean;
  }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { retry } = data;
  const { errors, isSubmitting } = formState;

  const onSubmit = async () => {
    if (!username) return;
    const { accounts } = await client.getUser({ username });
    const numberOfAccounts = Object.keys(accounts).length;
    if (numberOfAccounts === 0) {
      setError("username", { message: "Username doesn't exist" });
    } else {
      setData({ username, retry: false });
      nextStep();
    }
  };

  return (
    <div className="flex flex-col gap-6 w-full">
      {retry ? (
        <p className="text-typography-rose-600 text-center text-xl">
          The credential connected does not match the on-chain record.
          <br /> Please try again.
        </p>
      ) : null}
      <Input
        {...register("username", {
          onChange: ({ target }) => setValue("username", target.value.toLowerCase()),
          validate: (value) => {
            if (!value) return "Username is required";
            if (value.length > 15) return "Username must be at most 15 characters long";
            return true;
          },
        })}
        placeholder="Choose an username"
        onKeyDown={({ key }) => key === "Enter" && onSubmit()}
        error={errors.username?.message}
      />
      <DangoButton fullWidth onClick={onSubmit} isLoading={isSubmitting}>
        {retry ? "Choose credentials" : "Login"}
      </DangoButton>
    </div>
  );
};
