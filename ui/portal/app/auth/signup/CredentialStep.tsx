"use client";

import {
  Button,
  CheckCircleIcon,
  Input,
  Spinner,
  XCircleIcon,
  useDebounce,
  useWizard,
} from "@dango/shared";
import { usePublicClient } from "@leftcurve/react";
import { useQuery } from "@tanstack/react-query";
import { useForm } from "react-hook-form";

export const CredentialStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const { setError, register, watch, handleSubmit, setValue, formState } = useForm<{
    username: string;
  }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { errors, isSubmitting } = formState;

  const {
    refetch,
    data: isUsernameAvailable,
    isFetching,
  } = useQuery({
    enabled: false,
    queryKey: ["username", username],
    queryFn: async () => {
      if (!username) return null;
      const { accounts } = await client.getUser({ username });
      const isUsernameAvailable = !Object.keys(accounts).length;
      if (!isUsernameAvailable) setError("username", { message: "Username is not available" });
      return isUsernameAvailable;
    },
  });

  useDebounce(refetch, 300, [username]);

  const onSubmit = handleSubmit(async () => {
    setData({ username });
    nextStep();
  });

  return (
    <form onSubmit={onSubmit} className="flex flex-col w-full gap-4 md:gap-6">
      <Input
        {...register("username", {
          onChange: ({ target }) => setValue("username", target.value.toLowerCase()),
          validate: (value) => {
            if (!value || value.length > 15 || !/^[a-z0-9_]+$/.test(value)) {
              return "Username must be no more than 15 lowercase alphanumeric (a-z|0-9) or underscore";
            }
            return true;
          },
        })}
        classNames={{ description: "text-typography-green-400" }}
        placeholder="Choose an username"
        description={isUsernameAvailable ? "Username is available" : undefined}
        endContent={
          isFetching ? (
            <Spinner size="sm" color="white" />
          ) : isUsernameAvailable === null ? null : isUsernameAvailable ? (
            <CheckCircleIcon className="stroke-typography-green-400 stroke-2" />
          ) : (
            <XCircleIcon className="stroke-typography-pink-200 stroke-2" />
          )
        }
        error={errors.username?.message}
      />
      <Button type="submit" fullWidth isLoading={isSubmitting}>
        Choose username
      </Button>
    </form>
  );
};
