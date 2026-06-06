import { use, useMemo, useState } from "react";
import { useAccount, useConnectors, useSigningClient, useSubmitTx } from "@left-curve/store";
import { getUserEmbeddedEthereumWallet, getEntropyDetailsFromUser } from "@privy-io/js-sdk-core";
import { useQuery } from "@tanstack/react-query";

import {
  Button,
  Checkbox,
  createContext,
  ensureErrorMessage,
  ExpandOptions,
  IconAlert,
  IconButton,
  IconChecked,
  IconClose,
  IconEmail,
  IconKey,
  IconLeft,
  IconWallet,
  IconWarningTriangle,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { secp256k1ParsePubKey } from "@left-curve/crypto";
import { encodeBase64, encodeHex } from "@left-curve/encoding";
import { createKeyHash } from "@left-curve/sdk";
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

type AddKeyScreen =
  | "options"
  | "email-input"
  | "email-otp"
  | "wallets"
  | "public-key-warning"
  | "public-key-input"
  | "public-key-summary";

interface AddKeyState {
  screen: AddKeyScreen;
  email: string;
  publicKeyInput: string;
  isPending: boolean;
}

interface AddKeyActions {
  setScreen: (screen: AddKeyScreen) => void;
  setEmail: (email: string) => void;
  setPublicKeyInput: (publicKeyInput: string) => void;
  linkEmailKey: () => Promise<void>;
  addKey: (connectorId: string) => Promise<void>;
  addPublicKey: (publicKey: string) => Promise<void>;
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
  const [publicKeyInput, setPublicKeyInput] = useState("");
  const connectors = useConnectors();
  const { account, username, userIndex } = useAccount();
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
      invalidateKeys: [["user_keys"], ["quests", username]],
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
      invalidateKeys: [["user_keys"], ["quests", username]],
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

  const { mutateAsync: submitPublicKey, isPending: isAddingPublicKey } = useSubmitTx({
    mutation: {
      invalidateKeys: [["user_keys"], ["quests", username]],
      mutationFn: async (publicKeyInput: string) => {
        if (!account || !signingClient) throw new Error("We couldn't process the request");

        const publicKey = secp256k1ParsePubKey(publicKeyInput);
        if (!publicKey) {
          throw new Error(m["settings.keyManagement.publicKey.input.error"]());
        }

        const keyHash = createKeyHash(publicKey);
        const keyExists = userKeys?.some((k) => k.keyHash === keyHash);
        if (keyExists) {
          throw new KeyAlreadyExistsError();
        }

        await signingClient.updateKey({
          keyHash,
          sender: account.address,
          action: {
            insert: { secp256k1: encodeBase64(publicKey) },
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

  const addPublicKey = async (publicKeyInput: string) => {
    await submitPublicKey(publicKeyInput);
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
        state: {
          screen,
          email,
          publicKeyInput,
          isPending: isAddingKey || isLinkingEmailKey || isAddingPublicKey,
        },
        actions: {
          setScreen,
          setEmail,
          setPublicKeyInput,
          linkEmailKey: safeLinkEmailKey,
          addKey,
          addPublicKey,
        },
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
  description?: React.ReactNode;
  variant?: "key" | "warning" | "success";
}

function AddKeyHeader({ title, description, variant = "key" }: AddKeyHeaderProps) {
  const { hideModal } = useApp();
  const Icon =
    variant === "warning" ? IconWarningTriangle : variant === "success" ? IconChecked : IconKey;

  return (
    <div className="p-4 flex flex-col gap-4">
      <IconButton
        className="hidden md:block absolute right-2 top-2"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>
      <div
        className={twMerge(
          "w-12 h-12 rounded-full flex items-center justify-center",
          variant === "warning"
            ? "bg-utility-warning-100 text-utility-warning-500"
            : "bg-surface-secondary-green text-primitives-green-light-600",
        )}
      >
        <Icon />
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="h4-bold text-ink-primary-900">{title}</h3>
        {description ? (
          <p className="text-ink-tertiary-500 diatype-m-regular">{description}</p>
        ) : null}
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
    actions: { setScreen },
  } = use(AddKeyContext);

  return (
    <Button variant="secondary" fullWidth onClick={() => setScreen("wallets")}>
      <IconWallet />
      {m["auth.wallets"]()}
    </Button>
  );
}

function AddKeyWalletsPicker() {
  const {
    state: { isPending },
    actions: { addKey, setScreen },
  } = use(AddKeyContext);

  return (
    <div className="flex flex-col gap-4 w-full items-center">
      <AuthOptions action={addKey} isPending={isPending} />
      <Button size="sm" variant="link" onClick={() => setScreen("options")}>
        <IconLeft className="w-[22px] h-[22px]" />
        <span>{m["common.back"]()}</span>
      </Button>
    </div>
  );
}

function AddKeyAdvanced() {
  return (
    <ExpandOptions showOptionText={m["settings.keyManagement.advanced"]()} showLine>
      <AddKeyPublicKeyOption />
    </ExpandOptions>
  );
}

function AddKeyPublicKeyOption() {
  const {
    actions: { setScreen },
  } = use(AddKeyContext);

  return (
    <Button variant="secondary" fullWidth onClick={() => setScreen("public-key-warning")}>
      <IconKey />
      {m["settings.keyManagement.publicKey.option"]()}
    </Button>
  );
}

function AddKeyPublicKeyWarning() {
  const [acknowledgements, setAcknowledgements] = useState({
    generated: false,
    privateKey: false,
    authority: false,
  });
  const {
    actions: { setScreen },
  } = use(AddKeyContext);
  const canContinue = Object.values(acknowledgements).every(Boolean);
  const setAcknowledgement = (key: keyof typeof acknowledgements) => (checked: boolean) =>
    setAcknowledgements((current) => ({ ...current, [key]: checked }));

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="rounded-xl bg-surface-primary-red text-ink-tertiary-red shadow-account-card px-3 py-2 flex gap-2 diatype-sm-medium">
        <IconAlert className="w-5 h-5 shrink-0 text-primitives-red-light-600" />
        <p>{m["settings.keyManagement.publicKey.warning.scam"]()}</p>
      </div>
      <div className="flex flex-col gap-2">
        <Checkbox
          color="grey"
          radius="md"
          size="sm"
          checked={acknowledgements.generated}
          onChange={setAcknowledgement("generated")}
          label={m["settings.keyManagement.publicKey.warning.confirmations.generated"]()}
        />
        <Checkbox
          color="grey"
          radius="md"
          size="sm"
          checked={acknowledgements.privateKey}
          onChange={setAcknowledgement("privateKey")}
          label={m["settings.keyManagement.publicKey.warning.confirmations.privateKey"]()}
        />
        <Checkbox
          color="grey"
          radius="md"
          size="sm"
          checked={acknowledgements.authority}
          onChange={setAcknowledgement("authority")}
          label={m["settings.keyManagement.publicKey.warning.confirmations.authority"]()}
        />
      </div>
      <div className="flex flex-col gap-3">
        <div className="flex gap-3">
          <Button fullWidth variant="secondary" onClick={() => setScreen("options")}>
            {m["common.cancel"]()}
          </Button>
          <Button fullWidth isDisabled={!canContinue} onClick={() => setScreen("public-key-input")}>
            {m["common.continue"]()}
          </Button>
        </div>
      </div>
    </div>
  );
}

function AddKeyPublicKeyInput() {
  const {
    state: { publicKeyInput, isPending },
    actions: { setPublicKeyInput, setScreen },
  } = use(AddKeyContext);
  const parsedPublicKey = useMemo(() => secp256k1ParsePubKey(publicKeyInput), [publicKeyInput]);
  const showError = publicKeyInput.trim().length > 0 && !parsedPublicKey;

  return (
    <form
      className="flex flex-col gap-4 w-full"
      onSubmit={(event) => {
        event.preventDefault();
        if (!parsedPublicKey) return;
        setScreen("public-key-summary");
      }}
    >
      <div className="flex flex-col gap-1 relative text-ink-secondary-700">
        <label className="exposure-sm-italic text-ink-secondary-700" htmlFor="secp256k1-public-key">
          {m["settings.keyManagement.publicKey.input.label"]()}
        </label>
        <div
          className={twMerge(
            "relative w-full inline-flex tap-highlight-transparent flex-row items-start shadow-account-card gap-2 z-10",
            "bg-surface-secondary-rice hover:bg-surface-tertiary-rice border border-transparent active:border-surface-quaternary-rice",
            "px-4 py-[13px] rounded-lg min-h-[64px]",
            showError ? "border-status-fail" : null,
            isPending
              ? "pointer-events-none bg-surface-disabled-gray placeholder:text-fg-disabled text-fg-disabled"
              : null,
          )}
        >
          <textarea
            id="secp256k1-public-key"
            value={publicKeyInput}
            disabled={isPending}
            autoComplete="off"
            autoCapitalize="none"
            autoCorrect="off"
            spellCheck={false}
            placeholder={m["settings.keyManagement.publicKey.input.placeholder"]()}
            onChange={(event) => setPublicKeyInput(event.target.value)}
            className={twMerge(
              "flex-1 min-h-[38px] resize-y diatype-m-regular bg-transparent outline-none placeholder:text-ink-tertiary-500 text-ink-secondary-700 relative z-10",
              showError ? "text-status-fail" : null,
            )}
          />
        </div>
        {showError ? (
          <span className="diatype-sm-regular text-status-fail">
            {m["settings.keyManagement.publicKey.input.error"]()}
          </span>
        ) : null}
        {parsedPublicKey ? (
          <span className="diatype-sm-regular text-primitives-green-light-600">
            {m["settings.keyManagement.publicKey.input.valid"]()}
          </span>
        ) : null}
      </div>
      <div className="flex gap-3">
        <Button
          fullWidth
          type="button"
          variant="secondary"
          onClick={() => setScreen("public-key-warning")}
        >
          {m["common.back"]()}
        </Button>
        <Button
          fullWidth
          type="submit"
          isLoading={isPending}
          isDisabled={!parsedPublicKey || isPending}
        >
          {m["settings.keyManagement.publicKey.input.submit"]()}
        </Button>
      </div>
    </form>
  );
}

function AddKeyPublicKeySummary() {
  const {
    state: { publicKeyInput, isPending },
    actions: { addPublicKey, setScreen },
  } = use(AddKeyContext);
  const parsedPublicKey = useMemo(() => secp256k1ParsePubKey(publicKeyInput), [publicKeyInput]);
  const keyLabel = parsedPublicKey ? formatPublicKeySummary(parsedPublicKey) : "-";

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="rounded-lg border border-outline-secondary-gray bg-surface-secondary-rice shadow-account-card overflow-hidden diatype-sm-regular text-ink-secondary-700">
        <SummaryRow label={m["settings.keyManagement.publicKey.summary.key"]()} value={keyLabel} />
        <SummaryRow
          label={m["settings.keyManagement.publicKey.summary.type"]()}
          value={m["settings.keyManagement.publicKey.summary.typeValue"]()}
        />
        <SummaryRow
          label={m["settings.keyManagement.publicKey.summary.signedBy"]()}
          value={m["settings.keyManagement.publicKey.summary.signedByValue"]()}
        />
      </div>
      <div className="flex gap-3">
        <Button fullWidth variant="secondary" onClick={() => setScreen("public-key-input")}>
          {m["common.back"]()}
        </Button>
        <Button
          fullWidth
          isLoading={isPending}
          isDisabled={!parsedPublicKey || isPending}
          onClick={() => {
            addPublicKey(publicKeyInput).catch(() => undefined);
          }}
        >
          {m["settings.keyManagement.publicKey.summary.confirm"]()}
        </Button>
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[minmax(72px,1fr)_minmax(0,2fr)] gap-3 border-b border-outline-secondary-gray last:border-b-0 px-3 py-2">
      <span>{label}</span>
      <span className="text-right diatype-sm-bold truncate">{value}</span>
    </div>
  );
}

function formatPublicKeySummary(publicKey: Uint8Array) {
  const publicKeyHex = encodeHex(publicKey);
  return `${publicKeyHex.slice(0, 10)} ... ${publicKeyHex.slice(-4)}`;
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
  WalletsPicker: AddKeyWalletsPicker,
  Advanced: AddKeyAdvanced,
  PublicKeyWarning: AddKeyPublicKeyWarning,
  PublicKeyInput: AddKeyPublicKeyInput,
  PublicKeySummary: AddKeyPublicKeySummary,
  EmailInput: AddKeyEmailInput,
  EmailOtp: AddKeyEmailOtp,
  Context: AddKeyContext,
};
