import {
  ensureErrorMessage,
  IconWallet,
  useApp,
  useInputs,
  useMediaQuery,
  useWizard,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useConfig,
  useConnectors,
  usePublicClient,
  useSignin,
  useSubmitTx,
} from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { computeAddress, createAccountSalt } from "@left-curve/dango";
import { createKeyHash } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";
import { wait } from "@left-curve/dango/utils";

import {
  Button,
  CheckCircleIcon,
  Checkbox,
  ExpandOptions,
  IconAlert,
  IconLeft,
  Input,
  ResizerContainer,
  Spinner,
  Stepper,
  XCircleIcon,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { AuthCarousel } from "./AuthCarousel";
import { AuthOptions } from "./AuthOptions";
import { EmailCredential } from "./EmailCredential";
import { SocialCredential } from "./SocialCredential";
import { PasskeyCredential } from "./PasskeyCredential";

import { AccountType } from "@left-curve/dango/types";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { Address, Hex, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "@left-curve/store/types";
import type React from "react";

const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep, data } = useWizard<{
    username: string;
    email: string;
    view: string;
  }>();
  const { isConnected } = useAccount();
  const navigate = useNavigate();
  const { view, email } = data;

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <>
      <Mobile />
      <div className="h-screen w-screen flex items-center justify-center bg-surface-primary-rice text-ink-primary-900">
        <div className="flex items-center justify-center flex-1 min-w-fit">
          <ResizerContainer layoutId="signup" className="w-full max-w-[24.5rem]">
            <div className="flex items-center justify-center gap-8 px-4 flex-col w-full">
              <div className="flex flex-col gap-7 items-center justify-center w-full">
                <img
                  src="./favicon.svg"
                  alt="dango-logo"
                  className="h-12 rounded-full shadow-account-card"
                />
                {activeStep !== 3 ? (
                  <div className="flex flex-col gap-3 items-center justify-center text-center">
                    <h1 className="h2-heavy">{m["signup.stepper.title"]({ step: activeStep })}</h1>
                    {!["wallets", "email"].includes(view) ? (
                      <p className="text-ink-tertiary-500 diatype-m-medium">
                        {m["signup.stepper.description"]({ step: activeStep })}
                      </p>
                    ) : null}
                    {view === "wallets" ? (
                      <p className="text-ink-tertiary-500 diatype-m-medium">
                        {m["signin.connectWalletToContinue"]()}
                      </p>
                    ) : null}
                    {view === "email" ? (
                      <p className="text-ink-tertiary-500">
                        {m["signin.sentVerificationCode"]()}
                        <span className="font-bold">{email}</span>
                      </p>
                    ) : null}
                  </div>
                ) : (
                  <div className="flex flex-col gap-3 items-center justify-center text-center w-full">
                    <h1 className="h2-heavy">
                      {m["common.hi"]()}, {data.username}
                    </h1>
                    <p className="text-ink-tertiary-500 diatype-m-medium">
                      {m["signup.stepper.description"]({ step: activeStep })}
                    </p>
                  </div>
                )}
                <Stepper
                  steps={Array.from({ length: 4 }).map((_, step) =>
                    m["signup.stepper.steps"]({ step }),
                  )}
                  activeStep={activeStep}
                />
              </div>
              {children}
              {activeStep === 0 && !["wallets", "email"].includes(view) ? (
                <div className="w-full flex flex-col items-center gap-1">
                  <div className="flex items-center gap-1">
                    <p>{m["signup.alreadyHaveAccount"]()}</p>
                    <Button
                      variant="link"
                      autoFocus={false}
                      onClick={() => navigate({ to: "/signin" })}
                    >
                      {m["common.signin"]()}
                    </Button>
                  </div>
                </div>
              ) : null}
              {activeStep === 1 ? (
                <div className="flex items-center flex-col">
                  <Button
                    as={Link}
                    to="/"
                    variant="link"
                    className="text-primitives-red-light-400 hover:text-primitives-red-light-600"
                  >
                    {m["signup.doThisLater"]()}
                  </Button>
                  <Button
                    size="sm"
                    variant="link"
                    className="flex justify-center items-center"
                    onClick={() => previousStep()}
                  >
                    <IconLeft className="w-[22px] h-[22px]" />
                    <span>{m["common.back"]()}</span>
                  </Button>
                </div>
              ) : null}
            </div>
          </ResizerContainer>
        </div>
        <AuthCarousel />
      </div>
    </>
  );
};

const Mobile: React.FC = () => {
  const { history } = useRouter();
  return (
    <div className="md:hidden w-screen h-screen bg-primitives-gray-light-900/50 fixed top-0 left-0 z-50 flex items-center justify-center p-4">
      <div className="w-full flex flex-col items-center justify-start bg-surface-primary-rice text-ink-primary-900 rounded-xl border border-outline-secondary-gray max-w-96">
        <div className="flex flex-col gap-4 p-4 border-b border-b-secondary-gray">
          <div className="w-12 h-12 bg-error-100 rounded-full flex items-center justify-center">
            <IconAlert className="w-6 h-6 text-status-fail" />
          </div>
          <p className="h4-bold text-ink-primary-900">{m["signup.mobileModal.title"]()}</p>
          <p className="diatype-m-medium text-ink-tertiary-500">
            {m["signup.mobileModal.description"]()}
          </p>
        </div>
        <div className="p-4 w-full">
          <Button variant="secondary" fullWidth onClick={() => history.go(-1)}>
            {m["common.cancel"]()}
          </Button>
        </div>
      </div>
    </div>
  );
};

const Credential: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { toast } = useApp();
  const { nextStep, setData, reset, data } = useWizard();
  const connectors = useConnectors();
  const { view: activeView } = data;

  const { isPending, mutateAsync: createCredential } = useSubmitTx({
    toast: {
      error: (e) =>
        toast.error({ title: m["errors.failureRequest"](), description: ensureErrorMessage(e) }),
    },
    mutation: {
      mutationFn: async (connectorId: string) => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");
        const challenge = "Please sign this message to confirm your identity.";
        const { key, keyHash } = await (async () => {
          if (connectorId === "passkey") {
            try {
              return connector.createNewKey!(challenge);
            } catch (cause) {
              throw new Error(
                "Your device is not compatible with passkey or you cancelled the request",
                { cause },
              );
            }
          }

          const provider = await (
            connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
          ).getProvider();

          const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });

          const addressLowerCase = controllerAddress.toLowerCase() as Address;

          const key: Key = { ethereum: addressLowerCase };
          const keyHash = createKeyHash(addressLowerCase);

          return { key, keyHash };
        })();

        setData({ key, keyHash, connectorId, seed: Math.floor(Math.random() * 0x10000) });
        nextStep();
      },
    },
  });

  const emailCredential = (
    <EmailCredential
      onAuth={() => createCredential("privy")}
      email={data.email}
      setEmail={(email) => {
        setData({ email, view: "email" });
      }}
      goBack={reset}
    />
  );

  if (activeView === "wallets")
    return (
      <div className="flex flex-col gap-7 w-full items-center">
        <div className="flex flex-col gap-4 w-full items-center">
          <AuthOptions action={createCredential} isPending={isPending} />
          <Button size="sm" variant="link" onClick={reset}>
            <IconLeft className="w-[22px] h-[22px]" />
            <span>{m["common.back"]()}</span>
          </Button>
        </div>
      </div>
    );
  if (activeView === "email") return emailCredential;

  return (
    <div className="flex flex-col gap-6 w-full">
      {emailCredential}

      <div className="w-full flex items-center justify-center gap-3">
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
        <p className="min-w-fit text-ink-placeholder-400 uppercase">{m["common.or"]()}</p>
        <span className="h-[1px] bg-outline-secondary-gray flex-1 " />
      </div>

      <div className="flex flex-col items-center w-full gap-4">
        <SocialCredential onAuth={() => createCredential("privy")} signup />
        <PasskeyCredential
          onAuth={() => createCredential("passkey")}
          label={m["common.signWithPasskey"]({ action: "signup" })}
        />

        {isMd ? (
          <Button variant="secondary" fullWidth onClick={() => setData({ view: "wallets" })}>
            <IconWallet />
            {m["signin.connectWallet"]()}
          </Button>
        ) : null}
      </div>
    </div>
  );
};

const Username: React.FC = () => {
  const { nextStep, data, setData } = useWizard<{
    key: Key;
    keyHash: Hex;
    connectorId: string;
    seed: number;
    username: string;
  }>();

  const { toast } = useApp();

  const { register, inputs } = useInputs();

  const { value: username, error } = inputs.username || {};

  const { key, keyHash, connectorId, seed } = data;

  const config = useConfig();
  const client = usePublicClient();
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
      if (!username) return new Error(m["signin.errors.usernameRequired"]());
      if (error) throw error;
      const { accounts } = await client.getUser({ username });
      const isUsernameAvailable = !Object.keys(accounts).length;

      if (!isUsernameAvailable) throw new Error(m["signup.errors.usernameTaken"]());
      return isUsernameAvailable;
    },
  });

  const { isPending, mutateAsync: createAccount } = useSubmitTx({
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signup.errors.creatingAccount"](),
        }),
    },
    mutation: {
      mutationFn: async () => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");

        const { credential } = await connector.signArbitrary({
          primaryType: "Message" as const,
          message: {
            username,
            chain_id: config.chain.id,
          },
          types: {
            Message: [
              { name: "username", type: "string" },
              { name: "chain_id", type: "string" },
            ],
          },
        });
        if (!("standard" in credential)) throw new Error("error: signed with wrong credential");

        await registerUser(client, {
          key,
          keyHash,
          username,
          seed,
          signature: credential.standard.signature,
        });

        setData({ ...data, username });
        nextStep();
      },
    },
  });

  return (
    <div className="flex flex-col gap-6 w-full">
      <Input
        placeholder={
          <p className="flex gap-1 items-center justify-start">
            <span>{m["signin.placeholder"]()}</span>
            <span className="text-ink-secondary-rice exposure-m-italic group-data-[focus=true]:text-ink-tertiary-500">
              {m["common.username"]().toLowerCase()}
            </span>
          </p>
        }
        {...register("username", {
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
            <XCircleIcon className="stroke-primitives-red-light-400 stroke-2" />
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
        {m["signup.createAccount"]()}
      </Button>
    </div>
  );
};

const Signin: React.FC = () => {
  const navigate = useNavigate();
  const { done, data } = useWizard<{ username: string; connectorId: string }>();
  const { toast, settings, changeSettings } = useApp();
  const { useSessionKey } = settings;

  const { username, connectorId } = data;

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    session: useSessionKey && { expireAt: Date.now() + DEFAULT_SESSION_EXPIRATION },
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        }),
    },
    mutation: {
      onSuccess: () => {
        navigate({ to: "/" });
        done();
      },
    },
  });

  return (
    <div className="flex flex-col gap-6 w-full">
      <Button
        fullWidth
        onClick={() => connectWithConnector({ username, connectorId })}
        isLoading={isPending}
      >
        {m["common.signin"]()}
      </Button>
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
    </div>
  );
};

export const Fund: React.FC = () => {
  const { nextStep, data } = useWizard<{
    key: Key;
    keyHash: Hex;
    connectorId: string;
    seed: number;
    username: string;
  }>();
  const connectors = useConnectors();
  const client = usePublicClient();

  const { toast } = useApp();

  const { key, keyHash, connectorId, seed } = data;

  const { isPending, mutateAsync: requestFunds } = useSubmitTx({
    toast: {
      error: () =>
        toast.error({
          title: m["common.error"](),
          description: m["signup.errors.failedSendingFunds"](),
        }),
    },
    mutation: {
      mutationFn: async () => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");

        const { addresses } = await client.getAppConfig();
        const accountCodeHash = await client.getAccountTypeCodeHash({
          accountType: AccountType.Spot,
        });

        const salt = createAccountSalt({ key, keyHash, seed });
        const address = computeAddress({
          deployer: addresses.accountFactory,
          codeHash: accountCodeHash,
          salt,
        });

        const response = await fetch(`${window.dango.urls.faucetUrl}/${address}`);
        if (!response.ok) throw new Error(m["signup.errors.failedSendingFunds"]());

        nextStep();
      },
    },
  });

  return (
    <div className="flex flex-col gap-6 w-full">
      <Button fullWidth onClick={() => requestFunds()} isLoading={isPending}>
        {m["auth.requetFromFaucet"]()}
      </Button>
    </div>
  );
};

export const Signup = Object.assign(Container, {
  Credential,
  Fund,
  Username,
  Signin,
});
