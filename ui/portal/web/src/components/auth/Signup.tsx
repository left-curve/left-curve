import { ensureErrorMessage, useInputs, useWizard } from "@left-curve/applets-kit";
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
import { useApp } from "~/hooks/useApp";

import { computeAddress, createAccountSalt } from "@left-curve/dango";
import { createKeyHash } from "@left-curve/dango";
import { registerUser } from "@left-curve/dango/actions";
import { createWebAuthnCredential } from "@left-curve/dango/crypto";
import { encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { getNavigatorOS, getRootDomain } from "@left-curve/dango/utils";
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

import { AccountType } from "@left-curve/dango/types";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";
import { m } from "~/paraglide/messages";

import type { Address, Hex, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "@left-curve/store/types";
import type React from "react";

const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep, data } = useWizard<{ username: string }>();
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <>
      <Mobile />
      <div className="h-screen w-screen flex items-center justify-center bg-surface-primary-rice text-primary-900">
        <div className="flex items-center justify-center flex-1">
          <ResizerContainer layoutId="signup" className="w-full max-w-[22.5rem]">
            <div className="flex items-center justify-center gap-8 px-4 lg:px-0 flex-col w-full">
              <div className="flex flex-col gap-7 items-center justify-center w-full">
                <img
                  src="./favicon.svg"
                  alt="dango-logo"
                  className="h-12 rounded-full shadow-account-card"
                />
                {activeStep !== 2 ? (
                  <div className="flex flex-col gap-3 items-center justify-center text-center w-full">
                    <h1 className="h2-heavy">{m["signup.stepper.title"]({ step: activeStep })}</h1>
                    <p className="text-tertiary-500 diatype-m-medium">
                      {m["signup.stepper.description"]({ step: activeStep })}
                    </p>
                  </div>
                ) : (
                  <div className="flex flex-col gap-3 items-center justify-center text-center">
                    <h1 className="h2-heavy">
                      {m["common.hi"]()}, {data.username}
                    </h1>
                    <p className="text-tertiary-500 diatype-m-medium">
                      {m["signup.stepper.description"]({ step: activeStep })}
                    </p>
                  </div>
                )}
                <Stepper
                  steps={Array.from({ length: 3 }).map((_, step) =>
                    m["signup.stepper.steps"]({ step }),
                  )}
                  activeStep={activeStep}
                />
              </div>
              {children}
              {activeStep === 0 ? (
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
                    className="text-red-bean-400 hover:text-red-bean-600"
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
    <div className="md:hidden w-screen h-screen bg-gray-900/50 fixed top-0 left-0 z-50 flex items-center justify-center p-4">
      <div className="w-full flex flex-col items-center justify-start bg-surface-primary-rice text-primary-900 rounded-xl border border-secondary-gray max-w-96">
        <div className="flex flex-col gap-4 p-4 border-b border-b-secondary-gray">
          <div className="w-12 h-12 bg-error-100 rounded-full flex items-center justify-center">
            <IconAlert className="w-6 h-6 text-error-500" />
          </div>
          <p className="h4-bold text-primary-900">{m["signup.mobileModal.title"]()}</p>
          <p className="diatype-m-medium text-tertiary-500">
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
  const { toast } = useApp();
  const { nextStep, setData } = useWizard();
  const connectors = useConnectors();

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
              const { id, getPublicKey } = await createWebAuthnCredential({
                challenge: encodeUtf8(challenge),
                user: {
                  name: `${getNavigatorOS()} ${new Date().toLocaleString()}`,
                },
                rp: {
                  name: window.document.title,
                  id: getRootDomain(window.location.hostname),
                },
                authenticatorSelection: {
                  residentKey: "preferred",
                  requireResidentKey: false,
                  userVerification: "preferred",
                },
              });

              const publicKey = await getPublicKey();
              const key: Key = { secp256r1: encodeBase64(publicKey) };
              const keyHash = createKeyHash(id);

              return { key, keyHash };
            } catch (err) {
              throw new Error(
                "Your device is not compatible with passkey or you cancelled the request",
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
        setData({ key, keyHash, connectorId, seed: Math.floor(Math.random() * 0x100000000) });
        nextStep();
      },
    },
  });

  return <AuthOptions action={createCredential} isPending={isPending} mode="signup" />;
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
      error: () => toast.error({ title: m["signup.errors.creatingAccount"]() }),
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

        const response = await fetch(`${window.dango.urls.faucetUrl}/${address}`);
        if (!response.ok) throw new Error(m["signup.errors.failedSendingFunds"]());

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
            <span className="text-primary-rice exposure-m-italic group-data-[focus=true]:text-tertiary-500">
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

export const Signup = Object.assign(Container, {
  Credential,
  Username,
  Signin,
});
