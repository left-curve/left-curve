import { Button, Input, useWizard } from "@dango/shared";
import { usePublicClient } from "@leftcurve/react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";

export const CredentialStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const { setError, register, watch, setValue, handleSubmit, formState } = useForm<{
    username: string;
  }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { errors, isSubmitting } = formState;

  const onSubmit = handleSubmit(async () => {
    if (!username) return;
    const { accounts } = await client.getUser({ username });
    const numberOfAccounts = Object.keys(accounts).length;
    if (numberOfAccounts > 0) {
      setError("username", { message: "Username is already taken" });
    } else {
      setData({ username });
      nextStep();
    }
  });

  return (
    <form onSubmit={onSubmit} className="flex flex-col w-full gap-4 md:gap-6">
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
      <Button type="submit" fullWidth isLoading={isSubmitting}>
        Choose credentials
      </Button>
    </form>
  );
};
