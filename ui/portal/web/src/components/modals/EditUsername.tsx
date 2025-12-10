import { forwardRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { wait } from "@left-curve/dango/utils";

import {
  Button,
  IconButton,
  IconClose,
  IconErrorCircle,
  IconSuccessCircle,
  Input,
  Spinner,
  useApp,
  useInputs,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useConfig,
  usePublicClient,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import type { Address, UserIndexAndName } from "@left-curve/dango/types";

export const EditUsername = forwardRef((_props, _ref) => {
  const { hideModal } = useApp();
  const { setState } = useConfig();
  const { username, account } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { register, inputs, reset } = useInputs({
    initialValues: {
      editedUsername: username as string,
    },
  });

  const client = usePublicClient();

  const { value: editedUsername, error } = inputs.editedUsername || {};

  const {
    data: isUsernameAvailable = null,
    isFetching,
    error: errorMessage = error,
  } = useQuery({
    enabled: !!editedUsername && editedUsername !== `User #${account?.index}`,
    queryKey: ["username", editedUsername],
    queryFn: async ({ signal }) => {
      await wait(450);
      if (signal.aborted) return null;
      if (!editedUsername) return new Error(m["signin.errors.usernameRequired"]());
      if (error) throw error;
      const { accounts } = await client
        .getUser({ username: editedUsername })
        .catch(() => ({ accounts: {} }));
      const isUsernameAvailable = !Object.keys(accounts).length;

      if (!isUsernameAvailable) throw new Error(m["signup.errors.usernameTaken"]());
      return isUsernameAvailable;
    },
  });

  const { mutateAsync: changeUsername, isPending } = useSubmitTx({
    mutation: {
      onSuccess: () => {
        setState((x) => ({
          ...x,
          userIndexAndName: {
            ...(x.userIndexAndName as UserIndexAndName),
            name: editedUsername,
          },
        }));
        hideModal();
        reset();
      },
      mutationFn: async () =>
        await signingClient?.updateUsername({
          sender: account?.address as Address,
          username: editedUsername,
        }),
    },
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>
      <div className="flex flex-col gap-2">
        <h2 className="text-ink-primary-900 h4-bold w-full">
          {m["settings.session.username.editUsername"]()}
        </h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["settings.session.username.editDescription"]()}
        </p>
      </div>
      <form
        className="flex flex-col gap-6"
        onSubmit={(e) => {
          e.preventDefault();
          changeUsername();
        }}
      >
        <Input
          {...register("editedUsername", {
            strategy: "onChange",
            validate: (value) => {
              if (!value || value.length > 15 || !/^[a-z0-9_]+$/.test(value)) {
                return "Username must be no more than 15 lowercase alphanumeric (a-z|0-9) or underscore";
              }
              return true;
            },
            mask: (v) => v.toLowerCase(),
          })}
          errorMessage={
            errorMessage instanceof Error ? errorMessage.message : (errorMessage as string)
          }
          endContent={
            isFetching ? (
              <Spinner size="sm" color="gray" />
            ) : errorMessage ? (
              <IconErrorCircle className="text-primitives-red-light-400" />
            ) : isUsernameAvailable ? (
              <IconSuccessCircle className="text-status-success" />
            ) : null
          }
        />
        <Button
          fullWidth
          isLoading={isPending}
          isDisabled={inputs.editedUsername?.value === `User #${account?.index}` || !!errorMessage}
          type="submit"
        >
          {m["settings.session.username.save"]()}
        </Button>
      </form>
    </div>
  );
});
