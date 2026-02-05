import { use, useState } from "react";
import { useAccount, useConnectors, useSigningClient, useSubmitTx } from "@left-curve/store";
import { getUserEmbeddedEthereumWallet, getEntropyDetailsFromUser } from "@privy-io/js-sdk-core";
import { useQuery } from "@tanstack/react-query";

import {
  Button,
  createContext,
  ensureErrorMessage,
  ExpandOptions,
  IconButton,
  IconClose,
  IconEmail,
  IconKey,
  useApp,
} from "@left-curve/applets-kit";
import { createKeyHash } from "@left-curve/dango";
import { AuthOptions } from "../auth/AuthOptions";
import { PasskeyCredential } from "../auth/PasskeyCredential";
import { EmailCredential } from "../auth/EmailCredential";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { Connector } from "@left-curve/store/types";
import type Privy from "@privy-io/js-sdk-core";

class KeyAlreadyExistsError extends Error {
  constructor() {
    super("Key already exists");
    this.name = "KeyAlreadyExistsError";
  }
}

type AddKeyScreen = "options" | "email-input" | "email-otp";

interface AddKeyState {
  screen: AddKeyScreen;
  email: string;
  isPending: boolean;
}

interface AddKeyActions {
  setScreen: (screen: AddKeyScreen) => void;
  setEmail: (email: string) => void;
  linkEmailKey: () => Promise<void>;
  addKey: (connectorId: string) => Promise<void>;
}

interface AddKeyContextValue {
  state: AddKeyState;
  actions: AddKeyActions;
}

const [AddKeyContextProvider, , AddKeyContext] = createContext<AddKeyContextValue>({
  name: "AddKeyContext",
  strict: true,
  errorMessage: "AddKey components must be wrapped in AddKey.Provider",
});

function AddKeyProvider({ children }: { children: React.ReactNode }) {
  const [screen, setScreen] = useState<AddKeyScreen>("options");
  const [email, setEmail] = useState("");
  const connectors = useConnectors();
  const { account, userIndex } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { hideModal, toast } = useApp();

  const { data: userKeys } = useQuery({
    enabled: !!signingClient && !!userIndex,
    queryKey: ["user_keys", userIndex],
    queryFn: async () => await signingClient?.getUserKeys({ userIndex: userIndex! }),
  });

  const handleError = (error: unknown) => {
    if (error instanceof KeyAlreadyExistsError) {
      toast.error({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: m["settings.keyManagement.management.add.error.alreadyExists"](),
      });
    } else {
      toast.error({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: ensureErrorMessage(error),
      });
    }
  };

  const handleSuccess = () => {
    toast.success({
      title: m["settings.keyManagement.management.add.success.title"](),
      description: m["settings.keyManagement.management.add.success.description"](),
    });
    hideModal();
  };

  const { mutateAsync: submitKey, isPending: isAddingKey } = useSubmitTx({
    mutation: {
      invalidateKeys: [["user_keys"], ["quests", account?.username]],
      mutationFn: async (connectorId: string) => {
        const connector = connectors.find((c) => c.id === connectorId);
        if (!connector) throw new Error("Connector not found");
        if (!account || !signingClient) throw new Error("We couldn't process the request");

        const { keyHash, key } = await connector.createNewKey!();

        const keyExists = userKeys?.some((k) => k.keyHash === keyHash);
        if (keyExists) {
          throw new KeyAlreadyExistsError();
        }

        await signingClient?.updateKey({
          keyHash,
          sender: account.address,
          action: {
            insert: key,
          },
        });
      },
      onSuccess: handleSuccess,
      onError: handleError,
    },
  });

  const { mutateAsync: linkEmailKey, isPending: isLinkingEmailKey } = useSubmitTx({
    mutation: {
      invalidateKeys: [["user_keys"], ["quests", account?.username]],
      mutationFn: async () => {
        const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };
        if (!connector) throw new Error("Privy connector not found");
        if (!account || !signingClient) throw new Error("We couldn't process the request");

        const { user } = await connector.privy.user.get();
        if (!user) throw new Error("User not found");

        const wallet = getUserEmbeddedEthereumWallet(user);
        if (!wallet) throw new Error("No embedded wallet found");

        const { entropyId, entropyIdVerifier } = getEntropyDetailsFromUser(user) || {};
        if (!entropyId || !entropyIdVerifier) throw new Error("Could not get entropy details");

        const provider = await connector.privy.embeddedWallet.getEthereumProvider({
          wallet,
          entropyId,
          entropyIdVerifier,
        });

        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const addressLowerCase = controllerAddress.toLowerCase();
        const keyHash = createKeyHash(addressLowerCase);

        const keyExists = userKeys?.some((k) => k.keyHash === keyHash);
        if (keyExists) {
          throw new KeyAlreadyExistsError();
        }

        const key = { ethereum: addressLowerCase as `0x${string}` };

        await signingClient.updateKey({
          keyHash,
          sender: account.address,
          action: {
            insert: key,
          },
        });
      },
      onSuccess: handleSuccess,
      onError: handleError,
    },
  });

  const addKey = async (connectorId: string) => {
    await submitKey(connectorId);
  };

  const safeLinkEmailKey = async () => {
    try {
      await linkEmailKey();
    } catch {
      // Error already handled by onError callback
      // We catch here to prevent propagation to EmailCredential.OTP
    }
  };

  return (
    <AddKeyContextProvider
      value={{
        state: { screen, email, isPending: isAddingKey || isLinkingEmailKey },
        actions: { setScreen, setEmail, linkEmailKey: safeLinkEmailKey, addKey },
      }}
    >
      {children}
    </AddKeyContextProvider>
  );
}

function AddKeyFrame({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col bg-surface-primary-rice rounded-xl relative w-full min-w-[320px] md:min-w-[400px] max-w-[440px]">
      {children}
    </div>
  );
}

interface AddKeyHeaderProps {
  title: string;
  description: string;
}

function AddKeyHeader({ title, description }: AddKeyHeaderProps) {
  const { hideModal } = useApp();

  return (
    <div className="p-4 flex flex-col gap-4">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div className="w-12 h-12 rounded-full bg-surface-secondary-green flex items-center justify-center text-primitives-green-light-600">
        <IconKey />
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="h4-bold text-ink-primary-900">{title}</h3>
        <p className="text-ink-tertiary-500 diatype-m-regular">{description}</p>
      </div>
    </div>
  );
}

function AddKeyOptions({ children }: { children: React.ReactNode }) {
  return (
    <>
      <span className="w-full h-[1px] bg-outline-secondary-gray my-2" />
      <div className="flex flex-col gap-4 w-full p-4">{children}</div>
    </>
  );
}

function AddKeyPasskey() {
  const {
    actions: { addKey },
  } = use(AddKeyContext);

  return (
    <PasskeyCredential
      label={m["auth.passkey"]()}
      onAuth={() => addKey("passkey")}
      variant="primary"
    />
  );
}

function AddKeyEmail() {
  const {
    actions: { setScreen },
  } = use(AddKeyContext);

  return (
    <Button variant="secondary" fullWidth onClick={() => setScreen("email-input")}>
      <IconEmail />
      {m["auth.email"]()}
    </Button>
  );
}

function AddKeyWallets() {
  const {
    state: { isPending },
    actions: { addKey },
  } = use(AddKeyContext);

  return (
    <ExpandOptions showOptionText={m["auth.wallets"]()} showLine>
      <AuthOptions action={addKey} isPending={isPending} />
    </ExpandOptions>
  );
}

function AddKeyEmailInput() {
  const {
    state: { email },
    actions: { setEmail, setScreen },
  } = use(AddKeyContext);

  return (
    <EmailCredential.Email
      value={email}
      onChange={setEmail}
      onSubmit={() => setScreen("email-otp")}
    />
  );
}

function AddKeyEmailOtp() {
  const {
    state: { email },
    actions: { linkEmailKey, setScreen },
  } = use(AddKeyContext);

  return (
    <EmailCredential.OTP
      email={email}
      onSuccess={linkEmailKey}
      goBack={() => setScreen("email-input")}
    />
  );
}

export const AddKey = {
  Provider: AddKeyProvider,
  Frame: AddKeyFrame,
  Header: AddKeyHeader,
  Options: AddKeyOptions,
  Passkey: AddKeyPasskey,
  Email: AddKeyEmail,
  Wallets: AddKeyWallets,
  EmailInput: AddKeyEmailInput,
  EmailOtp: AddKeyEmailOtp,
  Context: AddKeyContext,
};
