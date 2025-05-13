import {
  Checkbox,
  ExpandOptions,
  IconAlert,
  IconLeft,
  ResizerContainer,
  Stepper,
  useUsernames,
  useWizard,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useConfig,
  useConnectors,
  usePublicClient,
  useSignin,
} from "@left-curve/store";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { computeAddress, createAccountSalt } from "@left-curve/dango";
import { createKeyHash } from "@left-curve/dango";
import { createWebAuthnCredential } from "@left-curve/dango/crypto";
import { encodeBase64, encodeUtf8 } from "@left-curve/dango/encoding";
import { getNavigatorOS, getRootDomain } from "@left-curve/dango/utils";

import { registerUser } from "@left-curve/dango/actions";
import { AccountType } from "@left-curve/dango/types";
import { wait } from "@left-curve/dango/utils";
import { toast } from "../foundation/Toast";

import {
  Button,
  CheckCircleIcon,
  Input,
  Spinner,
  XCircleIcon,
  useInputs,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import { AuthOptions } from "./AuthOptions";

import { m } from "~/paraglide/messages";

import type { Address, Hex, Key } from "@left-curve/dango/types";
import type { EIP1193Provider } from "@left-curve/store/types";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { AuthCarousel } from "./AuthCarousel";

import { captureException } from "@sentry/react";
import { DEFAULT_SESSION_EXPIRATION } from "~/constants";

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
      <div className="h-screen w-screen flex items-center justify-center">
        <div className="flex items-center justify-center flex-1">
          <ResizerContainer layoutId="signup" className="w-full max-w-[22.5rem]">
            <div className="flex items-center justify-center gap-8 px-4 lg:px-0 flex-col w-full">
              <div className="flex flex-col gap-7 items-center justify-center w-full">
                <img
                  src="./favicon.svg"
                  alt="dango-logo"
                  className="h-12 rounded-full shadow-btn-shadow-gradient"
                />
                {activeStep !== 2 ? (
                  <div className="flex flex-col gap-3 items-center justify-center text-center w-full">
                    <h1 className="h2-heavy">{m["signup.stepper.title"]({ step: activeStep })}</h1>
                    <p className="text-gray-500 diatype-m-medium">
                      {m["signup.stepper.description"]({ step: activeStep })}
                    </p>
                  </div>
                ) : (
                  <div className="flex flex-col gap-3 items-center justify-center text-center">
                    <h1 className="h2-heavy">
                      {m["common.hi"]()}, {data.username}
                    </h1>
                    <p className="text-gray-500 diatype-m-medium">
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
                  <Button
                    fullWidth
                    className="p-0 h-fit"
                    variant="link"
                    onClick={() => navigate({ to: "/forgot-username" })}
                  >
                    {m["signin.forgotUsername"]()}
                  </Button>
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
      <div className="w-full flex flex-col items-center justify-start bg-white-100 rounded-xl border border-gray-100 max-w-96">
        <div className="flex flex-col gap-4 p-4 border-b border-b-gray-100">
          <div className="w-12 h-12 bg-error-100 rounded-full flex items-center justify-center">
            <IconAlert className="w-6 h-6 text-error-500" />
          </div>
          <p className="h4-bold">{m["signup.mobileModal.title"]()}</p>
          <p className="diatype-m-medium text-gray-500">{m["signup.mobileModal.description"]()}</p>
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
  const { nextStep, setData } = useWizard();
  const connectors = useConnectors();

  const { isPending, mutateAsync: createCredential } = useMutation({
    mutationFn: async (connectorId: string) => {
      try {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("error: missing connector");
        const challenge = "Please sign this message to confirm your identity.";
        const { key, keyHash } = await (async () => {
          if (connectorId === "passkey") {
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
      } catch (err) {
        toast.error({ title: m["common.errorFailedRequest"]() });
        console.log(err);
      }
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

  const { isPending, mutateAsync: createAccount } = useMutation({
    mutationFn: async () => {
      try {
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

        const response = await fetch(`${import.meta.env.PUBLIC_FAUCET_URI}/mint/${address}`);
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
      } catch (err) {
        toast.error({ title: m["signup.errors.creatingAccount"]() });
        console.log(err);
        captureException(err, {
          data: {
            key,
            keyHash,
            username,
            connectorId,
            seed,
          },
        });
      }
    },
  });

  return (
    <div className="flex flex-col gap-6 w-full">
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
  const { addUsername } = useUsernames();
  const { done, data } = useWizard<{ username: string; connectorId: string }>();
  const { settings, changeSettings } = useApp();
  const { useSessionKey } = settings;

  const { username, connectorId } = data;

  const { mutateAsync: connectWithConnector, isPending } = useSignin({
    sessionKey: useSessionKey && { expireAt: Date.now() + DEFAULT_SESSION_EXPIRATION },
    mutation: {
      onSuccess: () => {
        navigate({ to: "/" });
        addUsername(username);
        done();
      },
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["signin.errors.failedSigningIn"](),
        });
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
