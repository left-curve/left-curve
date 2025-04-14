import { useInputs, useMediaQuery, useWizard } from "@left-curve/applets-kit";
import { useAccount, usePublicClient, useSignin } from "@left-curve/store";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { useApp } from "~/hooks/useApp";

import {
  Button,
  Checkbox,
  ExpandOptions,
  IconLeft,
  IconPasskey,
  IconQR,
  Input,
  ResizerContainer,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { Modals } from "../foundation/RootModal";
import { toast } from "../foundation/Toast";
import { AuthCarousel } from "./AuthCarousel";
import { AuthOptions } from "./AuthOptions";

import { m } from "~/paraglide/messages";

import type React from "react";
import type { FormEvent, PropsWithChildren } from "react";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-svh xl:h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer layoutId="signin" className="w-full max-w-[22.5rem]">
          {children}
        </ResizerContainer>
      </div>
      <AuthCarousel />
    </div>
  );
};

const UsernameStep: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { useSessionKey } = settings;

  const navigate = useNavigate();
  const { nextStep, setData } = useWizard<{ username: string; sessionKey: boolean }>();
  const { register, inputs, setError } = useInputs();
  const { isMd } = useMediaQuery();
  const { showModal } = useApp();

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
        setData({ username, sessionKey: useSessionKey });
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
              <span className="text-rice-800 exposure-m-italic group-data-[focus=true]:text-gray-500">
                {m["common.username"]().toLowerCase()}
              </span>
            </p>
          }
          {...register("username", {
            strategy: "onChange",
            validate: (value) => {
              if (!value) return m["signin.errors.usernameRequired"]();
              if (value.length > 15 || !/^[a-z0-9_]+$/.test(value)) {
                return "Username must be no more than 15 lowercase alphanumeric (a-z|0-9) or underscore";
              }
              return true;
            },
            mask: (v) => v.toLowerCase(),
          })}
        />
        <Button fullWidth type="submit" isDisabled={!!error} isLoading={isPending}>
          {m["common.signin"]()}
        </Button>
        {isMd ? (
          <Button as={Link} fullWidth variant="secondary" to="/">
            {m["signin.continueWithoutSignin"]()}
          </Button>
        ) : (
          <Button
            fullWidth
            onClick={() => showModal(Modals.SignWithDesktop)}
            className="gap-2"
            variant="secondary"
          >
            <IconQR className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signinWithDesktop"]()}</p>
          </Button>
        )}
        {isMd ? (
          <ExpandOptions showOptionText={m["signin.advancedOptions"]()}>
            <div className="flex items-center gap-2 flex-col">
              <Checkbox
                size="md"
                label={m["common.signinWithSession"]()}
                checked={useSessionKey}
                onChange={(v) => changeSettings({ useSessionKey: v })}
              />
            </div>
          </ExpandOptions>
        ) : null}
      </form>
      <div className="flex flex-col items-center">
        {isMd ? (
          <div className="flex justify-center items-center">
            <p>{m["signin.noAccount"]()}</p>
            <Button variant="link" onClick={() => navigate({ to: "/signup" })}>
              {m["common.signup"]()}
            </Button>
          </div>
        ) : (
          <Button as={Link} fullWidth variant="link" to="/">
            {m["signin.continueWithoutSignin"]()}
          </Button>
        )}
      </div>
    </div>
  );
};

const CredentialStep: React.FC = () => {
  const navigate = useNavigate();
  const { data, previousStep } = useWizard<{ username: string; sessionKey: boolean }>();
  const { isMd } = useMediaQuery();

  const { username, sessionKey } = data;

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    username,
    sessionKey,
    mutation: {
      onSuccess: () => navigate({ to: "/" }),
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
        previousStep();
      },
    },
  });

  return (
    <>
      <div className="flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
        <div className="flex flex-col gap-7 items-center justify-center">
          <img
            src="./favicon.svg"
            alt="dango-logo"
            className="h-12 rounded-full shadow-btn-shadow-gradient"
          />
          <div className="flex flex-col gap-3 items-center justify-center text-center">
            <h1 className="h2-heavy">
              {m["common.hi"]()}, {username}
            </h1>
            <p className="text-gray-500 diatype-m-medium">{m["signin.credential.description"]()}</p>
          </div>
        </div>
        {isMd ? (
          <AuthOptions
            action={(connectorId) => connectWithConnector({ connectorId })}
            isPending={isPending}
            mode="signin"
          />
        ) : (
          <Button
            fullWidth
            onClick={() => connectWithConnector({ connectorId: "passkey" })}
            isLoading={isPending}
            className="gap-2"
          >
            <IconPasskey className="w-6 h-6" />
            <p className="min-w-20"> {m["common.signWithPasskey"]({ action: "signin" })}</p>
          </Button>
        )}
        <div className="flex items-center">
          <Button variant="link" onClick={() => previousStep()}>
            <IconLeft className="w-[22px] h-[22px] text-blue-500" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      </div>
    </>
  );
};

export const Signin = Object.assign(Container, {
  Username: UsernameStep,
  Credential: CredentialStep,
});
