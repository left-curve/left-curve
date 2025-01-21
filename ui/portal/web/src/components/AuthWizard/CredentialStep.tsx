import {
  Button,
  CheckCircleIcon,
  Input,
  Spinner,
  XCircleIcon,
  useDebounce,
  useWizard,
} from "@left-curve/portal-shared";
import { usePublicClient } from "@left-curve/react";
import { useQuery } from "@tanstack/react-query";
import { useForm } from "react-hook-form";

export const CredentialStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const { register, watch, handleSubmit, setValue, formState } = useForm<{
    username: string;
  }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { errors, isSubmitting } = formState;

  const {
    refetch,
    data: isUsernameAvailable = null,
    isFetching,
    error,
  } = useQuery({
    enabled: false,
    queryKey: ["username", username],
    queryFn: async () => {
      if (!username) return null;
      const { accounts } = await client.getUser({ username });
      const isUsernameAvailable = !Object.keys(accounts).length;
      if (!isUsernameAvailable) throw new Error("Username is not available");
      return isUsernameAvailable;
    },
  });

  useDebounce(
    () => {
      if (errors.username) return;
      refetch();
    },
    300,
    [username],
  );

  const onSubmit = handleSubmit(async () => {
    setData({ username });
    nextStep();
  });

  const errorMessage = errors.username?.message || error?.message;

  return (
    <form onSubmit={onSubmit} className="flex flex-col w-full gap-4">
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
        placeholder="Choose an username"
        isValid={!!isUsernameAvailable}
        endContent={
          isFetching ? (
            <Spinner size="sm" color="white" />
          ) : errorMessage ? (
            <XCircleIcon className="stroke-typography-pink-200 stroke-2" />
          ) : isUsernameAvailable ? (
            <CheckCircleIcon className="stroke-typography-green-400 stroke-2" />
          ) : null
        }
        errorMessage={errorMessage}
      />
      <Button type="submit" fullWidth isLoading={isSubmitting} isDisabled={!!errorMessage}>
        Choose username
      </Button>
    </form>
  );
};
