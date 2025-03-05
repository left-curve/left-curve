import { useEffect } from "react";

import { useAccount, useChainId, useConnectors, usePublicClient } from "@left-curve/store-react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { computeAddress, createAccountSalt } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";
import { AccountType } from "@left-curve/dango/types";
import { wait } from "@left-curve/dango/utils";
import { ConnectionStatus } from "@left-curve/store-react/types";
import { useToast } from "../Toast";

import {
  Button,
  CheckCircleIcon,
  Input,
  Spinner,
  XCircleIcon,
  useInputs,
  useWizard,
} from "@left-curve/applets-kit";

import type { AppConfig, Hex, Key } from "@left-curve/dango/types";
import type React from "react";

export const SignupUsernameStep: React.FC = () => {
  const { done, data } = useWizard<{ key: Key; keyHash: Hex; connectorId: string }>();
  const { register, inputs } = useInputs();

  const { value: username, error } = inputs.username || {};

  const { key, keyHash, connectorId } = data;

  const navigate = useNavigate();
  const { toast } = useToast();

  const client = usePublicClient();
  const { status } = useAccount();
  const chainId = useChainId();
  const connectors = useConnectors();

  const {
    data: isUsernameAvailable = null,
    isFetching,
    error: errorMessage = error,
  } = useQuery({
    enabled: !!username,
    queryKey: ["username", username],
    queryFn: async ({ signal }) => {
      await wait(450);
      if (signal.aborted) return null;
      if (!username || error) return null;
      const { accounts } = await client.getUser({ username });
      const isUsernameAvailable = !Object.keys(accounts).length;

      if (!isUsernameAvailable) throw new Error("Username is already taken");
      return isUsernameAvailable;
    },
  });

  const { isPending, mutateAsync: createAccount } = useMutation({
    mutationFn: async () => {
      try {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");

        const { addresses } = await client.getAppConfig<AppConfig>();
        const accountCodeHash = await client.getAccountTypeCodeHash({
          accountType: AccountType.Spot,
        });

        const secret = Math.floor(Math.random() * 0x100000000);

        const salt = createAccountSalt({ key, keyHash, secret });
        const address = computeAddress({
          deployer: addresses.accountFactory,
          codeHash: accountCodeHash,
          salt,
        });

        const response = await fetch("https://mock-warp.left-curve.workers.dev", {
          method: "POST",
          body: JSON.stringify({ address }),
        });
        if (!response.ok) throw new Error("error: failed to send funds");
        await registerUser(client, { key, keyHash, username, secret });

        await wait(1000);
        await connector.connect({ username, chainId, keyHash });
      } catch (err) {
        toast.error({ title: "Couldn't complete the request" });
        console.log(err);
      }
    },
  });

  useEffect(() => {
    if (status !== ConnectionStatus.Connected) return;
    navigate({ to: "/" });
    return () => done();
  }, [navigate, status]);

  return (
    <div className="flex flex-col gap-6 w-full">
      <Input
        label="Username"
        placeholder="Enter your username"
        {...register("username", {
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
            <XCircleIcon className="stroke-red-bean-400 stroke-2" />
          ) : isUsernameAvailable ? (
            <CheckCircleIcon className="stroke-status-success stroke-2" />
          ) : null
        }
      />
      <Button
        fullWidth
        onClick={() => createAccount()}
        isLoading={isPending}
        isDisabled={!isUsernameAvailable || !!errorMessage}
      >
        Create Account
      </Button>
    </div>
  );
};
