import { Button, Input, useWizard } from "@dango/shared";
import { usePublicClient } from "@left-curve/react";
import type React from "react";
import { useForm } from "react-hook-form";

export const LoginStep: React.FC = () => {
  const { nextStep, setData, data } = useWizard();
  const { setError, register, setValue, handleSubmit, formState } = useForm<{
    username: string;
    retry: boolean;
  }>({
    mode: "onChange",
  });
  const client = usePublicClient();

  const { retry } = data;
  const { errors, isSubmitting } = formState;

  const errorMessage =
    errors.username?.message || retry
      ? "The credential connected does not match the on-chain record. Please try again."
      : undefined;

  const onSubmit = handleSubmit(async ({ username }) => {
    if (!username) return;
    const { accounts } = await client.getUser({ username });
    const numberOfAccounts = Object.keys(accounts).length;
    if (numberOfAccounts === 0) {
      setError("username", { message: "Username doesn't exist" });
    } else {
      setData({ username, retry: false });
      nextStep();
    }
  });

  return (
    <form className="flex flex-col w-full gap-4" onSubmit={onSubmit}>
      <Input
        {...register("username", {
          onChange: ({ target }) => setValue("username", target.value.toLowerCase()),
          validate: (value) => {
            if (!value) return "Username is required";
            if (value.length > 15) return "Username must be at most 15 characters long";
            return true;
          },
        })}
        placeholder="Enter your username"
        onKeyDown={({ key }) => key === "Enter" && onSubmit()}
        errorMessage={errorMessage}
      />
      <Button fullWidth onClick={onSubmit} isLoading={isSubmitting}>
        {retry ? "Confirm" : "Login"}
      </Button>
    </form>
  );
};
