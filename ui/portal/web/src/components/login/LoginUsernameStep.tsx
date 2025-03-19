import { useInputs, useWizard } from "@left-curve/applets-kit";
import { usePublicClient } from "@left-curve/store-react";
import { Link, useNavigate } from "@tanstack/react-router";

import { Button, Input } from "@left-curve/applets-kit";

import { useMutation } from "@tanstack/react-query";
import type React from "react";
import type { FormEvent } from "react";
import { m } from "~/paraglide/messages";

export const LoginUsernameStep: React.FC = () => {
  const navigate = useNavigate();
  const { nextStep, setData, data } = useWizard<{ username: string }>();
  const { register, inputs, setError } = useInputs();

  const { value: username, error } = inputs.username || {};

  const client = usePublicClient();

  const { mutateAsync: signInWithUsername, isPending } = useMutation({
    mutationFn: async (e: FormEvent<HTMLFormElement>) => {
      e.preventDefault();
      if (!username) return;
      const { accounts } = await client.getUser({ username });
      const numberOfAccounts = Object.keys(accounts).length;
      if (numberOfAccounts === 0) {
        setError("username", m["signin.errors.usernameNotExist"]());
      } else {
        setData({ username });
        nextStep();
      }
    },
  });

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-btn-shadow-gradient"
        />
        <h1 className="h2-heavy">{m["common.signin"]()}</h1>
      </div>
      <form className="flex flex-col gap-6 w-full" onSubmit={signInWithUsername}>
        <Input
          placeholder={
            <p className="flex gap-1 items-center justify-start">
              <span>{m["signin.placeholder"]()}</span>
              <span className="text-rice-800 exposure-m-italic group-data-[focus=true]:text-gray-500 group-data-[focus=true]:diatype-m-regular group-data-[focus=true]:not-italic">
                {m["common.username"]().toLowerCase()}
              </span>
            </p>
          }
          {...register("username", {
            validate: (value) => {
              if (!value) return m["signin.errors.usernameRequired"]();
              return true;
            },
            mask: (v) => v.toLowerCase(),
          })}
        />
        <Button fullWidth type="submit" isDisabled={!!error} isLoading={isPending}>
          {m["common.signin"]()}
        </Button>
        <Button as={Link} fullWidth variant="secondary" to="/">
          {m["signin.continueWithoutLogin"]()}
        </Button>
      </form>
      <div className="flex items-center">
        <p>{m["signin.noAccount"]()}</p>
        <Button variant="link" onClick={() => navigate({ to: "/signup" })}>
          {m["common.signup"]()}
        </Button>
      </div>
    </div>
  );
};
