import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { use, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Secp256k1 } from "@left-curve/crypto";
import { encodeBase64, encodeHex } from "@left-curve/encoding";
import { createKeyHash } from "@left-curve/sdk";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { resetAppletsKitMocks, setAppletsKitUseApp } from "../../../tests/mocks/applets-kit";

import { AddKey } from "./AddKey";
import { AddKeyModal } from "./AddKeyModal";

const mocks = vi.hoisted(() => ({
  setScreen: vi.fn(),
  toastError: vi.fn(),
  hideModal: vi.fn(),
  useAccount: vi.fn(),
  useConnectors: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
  useQuery: vi.fn(),
  getEntropyDetailsFromUser: vi.fn(),
  getUserEmbeddedEthereumWallet: vi.fn(),
}));

vi.mock("../auth/AuthOptions", () => ({
  AuthOptions: () => null,
}));

vi.mock("../auth/EmailCredential", () => ({
  EmailCredential: {
    Email: () => null,
    OTP: () => null,
  },
}));

vi.mock("../auth/PasskeyCredential", () => ({
  PasskeyCredential: () => null,
}));

vi.mock("@left-curve/store", () => ({
  useAccount: mocks.useAccount,
  useConnectors: mocks.useConnectors,
  useSigningClient: mocks.useSigningClient,
  useSubmitTx: mocks.useSubmitTx,
}));

vi.mock("~/constants", () => ({
  PRIVY_ERRORS_MAPPING: {},
}));

vi.mock("@privy-io/js-sdk-core", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@privy-io/js-sdk-core")>()),
  getEntropyDetailsFromUser: mocks.getEntropyDetailsFromUser,
  getUserEmbeddedEthereumWallet: mocks.getUserEmbeddedEthereumWallet,
}));

vi.mock("@tanstack/react-query", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-query")>();

  return {
    ...actual,
    useQuery: mocks.useQuery,
  };
});

describe("AddKey public key flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: mocks.hideModal,
      toast: {
        error: mocks.toastError,
        success: vi.fn(),
      },
    });
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: vi.fn(() => null),
        setItem: vi.fn(),
        removeItem: vi.fn(),
      },
    });
    mocks.useAccount.mockReturnValue({
      account: { address: "0x73656e6465720000000000000000000000000000" },
      username: "alice",
      userIndex: 0,
    });
    mocks.useConnectors.mockReturnValue([]);
    mocks.getEntropyDetailsFromUser.mockReturnValue(undefined);
    mocks.getUserEmbeddedEthereumWallet.mockReturnValue(undefined);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey: vi.fn(),
      },
    });
    mocks.useQuery.mockReturnValue({
      data: [],
      isPending: false,
    });
    mocks.useSubmitTx.mockImplementation(({ mutation }) => ({
      isPending: false,
      mutateAsync: async (value: unknown) => {
        try {
          const result = await mutation.mutationFn(value);
          mutation.onSuccess?.(result);
          return result;
        } catch (error) {
          mutation.onError?.(error);
          throw error;
        }
      },
    }));
  });

  it("blocks invalid public key submission on the input form", () => {
    const { container } = render(<PublicKeyInputHarness />);

    const publicKeyInput = container.querySelector<HTMLTextAreaElement>("#secp256k1-public-key")!;
    const submitButton = container.querySelector<HTMLButtonElement>("button[type='submit']")!;

    expect(publicKeyInput).not.toBeNull();
    expect(submitButton).not.toBeNull();
    fireEvent.change(publicKeyInput, { target: { value: "not a key" } });

    expect(submitButton).toBeDisabled();
    fireEvent.submit(publicKeyInput.closest("form")!);

    expect(mocks.setScreen).not.toHaveBeenCalledWith("public-key-summary");
  });

  it("registers key-list invalidation metadata for every add-key mutation", () => {
    render(
      <AddKey.Provider>
        <div>add-key metadata probe</div>
      </AddKey.Provider>,
    );

    const mutationConfigs = mocks.useSubmitTx.mock.calls.map(([parameters]) => parameters.mutation);

    expect(mutationConfigs).toHaveLength(3);
    for (const mutation of mutationConfigs) {
      expect(mutation.invalidateKeys).toEqual([["user_keys"]]);
    }
  });

  it("shows the duplicate-key toast when a public key already exists", async () => {
    const updateKey = vi.fn();
    const publicKey = new Secp256k1(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1),
    ).getPublicKey(true);

    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });
    mocks.useQuery.mockReturnValue({
      data: [{ keyHash: createKeyHash(publicKey) }],
      isPending: false,
    });

    render(
      <AddKey.Provider>
        <SubmitPublicKey publicKey={publicKey} />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit duplicate public key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledOnce();
    });
    expect(updateKey).not.toHaveBeenCalled();
  });

  it("adds connector-created keys through the signing client", async () => {
    const updateKey = vi.fn();
    const createNewKey = vi.fn().mockResolvedValue({
      keyHash: "0x706173736b65792d6164646564000000000000000000000000000000000000",
      key: { secp256r1: "passkey-public-key" },
    });

    mocks.useConnectors.mockReturnValue([
      {
        id: "passkey",
        createNewKey,
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitConnectorKey connectorId="passkey" label="Submit connector key" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit connector key" }));

    await waitFor(() => {
      expect(updateKey).toHaveBeenCalledWith({
        action: {
          insert: {
            secp256r1: "passkey-public-key",
          },
        },
        keyHash: "0x706173736b65792d6164646564000000000000000000000000000000000000",
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });
    expect(createNewKey).toHaveBeenCalledOnce();
    expect(mocks.hideModal).toHaveBeenCalledOnce();
  });

  it("does not create or submit connector keys without a signing client", async () => {
    const updateKey = vi.fn();
    const createNewKey = vi.fn().mockResolvedValue({
      keyHash: "0x706173736b65792d6d697373696e6700000000000000000000000000000000",
      key: { secp256r1: "passkey-public-key" },
    });

    mocks.useConnectors.mockReturnValue([
      {
        id: "passkey",
        createNewKey,
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    render(
      <AddKey.Provider>
        <SubmitConnectorKey connectorId="passkey" label="Submit connector without signing" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit connector without signing" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "We couldn't process the request",
      });
    });
    expect(createNewKey).not.toHaveBeenCalled();
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not submit public keys without a signing client", async () => {
    const updateKey = vi.fn();
    const publicKey = new Secp256k1(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1),
    ).getPublicKey(true);

    mocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    render(
      <AddKey.Provider>
        <SubmitPublicKey publicKey={publicKey} label="Submit public key without signing" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit public key without signing" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "We couldn't process the request",
      });
    });
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("surfaces backend public-key update failures without closing the modal", async () => {
    const updateKey = vi.fn().mockRejectedValue(new Error("backend public key rejected"));
    const publicKey = new Secp256k1(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1),
    ).getPublicKey(true);

    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitPublicKey publicKey={publicKey} label="Submit backend rejecting public key" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit backend rejecting public key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "backend public key rejected",
      });
    });
    expect(updateKey).toHaveBeenCalledWith({
      action: {
        insert: {
          secp256k1: encodeBase64(publicKey),
        },
      },
      keyHash: createKeyHash(publicKey),
      sender: "0x73656e6465720000000000000000000000000000",
    });
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("does not resolve Privy email keys without a signing client", async () => {
    const updateKey = vi.fn();
    const privyUserGet = vi.fn().mockResolvedValue({ user: { id: "privy-user-id" } });
    const getEthereumProvider = vi.fn();

    mocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          user: {
            get: privyUserGet,
          },
          embeddedWallet: {
            getEthereumProvider,
          },
        },
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    render(
      <AddKey.Provider>
        <SubmitEmailKey />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit email key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "We couldn't process the request",
      });
    });
    expect(privyUserGet).not.toHaveBeenCalled();
    expect(mocks.getUserEmbeddedEthereumWallet).not.toHaveBeenCalled();
    expect(mocks.getEntropyDetailsFromUser).not.toHaveBeenCalled();
    expect(getEthereumProvider).not.toHaveBeenCalled();
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("shows the duplicate-key toast when a connector-created key already exists", async () => {
    const updateKey = vi.fn();
    const existingKeyHash = "0x706173736b65792d6475700000000000000000000000000000000000000000";
    const createNewKey = vi.fn().mockResolvedValue({
      keyHash: existingKeyHash,
      key: { secp256r1: "passkey-public-key" },
    });

    mocks.useConnectors.mockReturnValue([
      {
        id: "passkey",
        createNewKey,
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });
    mocks.useQuery.mockReturnValue({
      data: [{ keyHash: existingKeyHash }],
      isPending: false,
    });

    render(
      <AddKey.Provider>
        <SubmitConnectorKey connectorId="passkey" label="Submit duplicate connector key" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit duplicate connector key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: m["settings.keyManagement.management.add.error.alreadyExists"](),
      });
    });
    expect(createNewKey).toHaveBeenCalledOnce();
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("surfaces connector key creation failures without updating account keys", async () => {
    const updateKey = vi.fn();
    const createNewKey = vi.fn().mockRejectedValue(new Error("wallet rejected"));

    mocks.useConnectors.mockReturnValue([
      {
        id: "passkey",
        createNewKey,
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitConnectorKey connectorId="passkey" label="Submit rejecting connector key" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit rejecting connector key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "wallet rejected",
      });
    });
    expect(createNewKey).toHaveBeenCalledOnce();
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("surfaces backend connector-key update failures without closing the modal", async () => {
    const updateKey = vi.fn().mockRejectedValue(new Error("backend key update rejected"));
    const createNewKey = vi.fn().mockResolvedValue({
      keyHash: "0x706173736b65792d6261636b656e642d6661696c65640000000000000000",
      key: { secp256r1: "passkey-public-key" },
    });

    mocks.useConnectors.mockReturnValue([
      {
        id: "passkey",
        createNewKey,
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitConnectorKey connectorId="passkey" label="Submit backend rejecting connector key" />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit backend rejecting connector key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "backend key update rejected",
      });
    });
    expect(createNewKey).toHaveBeenCalledOnce();
    expect(updateKey).toHaveBeenCalledWith({
      action: {
        insert: {
          secp256r1: "passkey-public-key",
        },
      },
      keyHash: "0x706173736b65792d6261636b656e642d6661696c65640000000000000000",
      sender: "0x73656e6465720000000000000000000000000000",
    });
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("adds an email-linked Privy key as a lowercased Ethereum key", async () => {
    const updateKey = vi.fn();
    const wallet = { address: "0xEMBEDDED" };
    const user = { id: "privy-user-id" };
    const provider = {
      request: vi.fn().mockResolvedValue(["0xABCDEFabcdefABCDEFabcdefABCDEFabcdefabcd"]),
    };
    const controllerAddress = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";

    mocks.getUserEmbeddedEthereumWallet.mockReturnValue(wallet);
    mocks.getEntropyDetailsFromUser.mockReturnValue({
      entropyId: "entropy-id",
      entropyIdVerifier: "email",
    });
    mocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          user: {
            get: vi.fn().mockResolvedValue({ user }),
          },
          embeddedWallet: {
            getEthereumProvider: vi.fn().mockResolvedValue(provider),
          },
        },
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitEmailKey />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit email key" }));

    await waitFor(() => {
      expect(updateKey).toHaveBeenCalledWith({
        action: {
          insert: {
            ethereum: controllerAddress,
          },
        },
        keyHash: createKeyHash(controllerAddress),
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });
    expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" });
    expect(mocks.hideModal).toHaveBeenCalledOnce();
  });

  it("shows the duplicate-key toast when an email-linked Privy key already exists", async () => {
    const updateKey = vi.fn();
    const wallet = { address: "0xEMBEDDED" };
    const user = { id: "privy-user-id" };
    const provider = {
      request: vi.fn().mockResolvedValue(["0xABCDEFabcdefABCDEFabcdefABCDEFabcdefabcd"]),
    };
    const controllerAddress = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";
    const existingKeyHash = createKeyHash(controllerAddress);

    mocks.getUserEmbeddedEthereumWallet.mockReturnValue(wallet);
    mocks.getEntropyDetailsFromUser.mockReturnValue({
      entropyId: "entropy-id",
      entropyIdVerifier: "email",
    });
    mocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          user: {
            get: vi.fn().mockResolvedValue({ user }),
          },
          embeddedWallet: {
            getEthereumProvider: vi.fn().mockResolvedValue(provider),
          },
        },
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });
    mocks.useQuery.mockReturnValue({
      data: [{ keyHash: existingKeyHash }],
      isPending: false,
    });

    render(
      <AddKey.Provider>
        <SubmitEmailKey />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit email key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: m["settings.keyManagement.management.add.error.alreadyExists"](),
      });
    });
    expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" });
    expect(updateKey).not.toHaveBeenCalled();
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("surfaces backend email-key update failures without closing the modal", async () => {
    const updateKey = vi.fn().mockRejectedValue(new Error("backend email key rejected"));
    const wallet = { address: "0xEMBEDDED" };
    const user = { id: "privy-user-id" };
    const provider = {
      request: vi.fn().mockResolvedValue(["0xABCDEFabcdefABCDEFabcdefABCDEFabcdefabcd"]),
    };
    const controllerAddress = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd";

    mocks.getUserEmbeddedEthereumWallet.mockReturnValue(wallet);
    mocks.getEntropyDetailsFromUser.mockReturnValue({
      entropyId: "entropy-id",
      entropyIdVerifier: "email",
    });
    mocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          user: {
            get: vi.fn().mockResolvedValue({ user }),
          },
          embeddedWallet: {
            getEthereumProvider: vi.fn().mockResolvedValue(provider),
          },
        },
      },
    ]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(
      <AddKey.Provider>
        <SubmitEmailKey />
      </AddKey.Provider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Submit email key" }));

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith({
        title: m["settings.keyManagement.management.add.error.title"](),
        description: "backend email key rejected",
      });
    });
    expect(provider.request).toHaveBeenCalledWith({ method: "eth_requestAccounts" });
    expect(updateKey).toHaveBeenCalledWith({
      action: {
        insert: {
          ethereum: controllerAddress,
        },
      },
      keyHash: createKeyHash(controllerAddress),
      sender: "0x73656e6465720000000000000000000000000000",
    });
    expect(mocks.hideModal).not.toHaveBeenCalled();
  });

  it("adds a public key through the modal warning, input, and summary screens", async () => {
    const updateKey = vi.fn();
    const publicKey = new Secp256k1(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1),
    ).getPublicKey(true);
    const publicKeyInput = `0x${encodeHex(publicKey)}`;

    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey,
      },
    });

    render(<AddKeyModal />);

    expect(
      screen.getByText(m["settings.keyManagement.management.add.title"]()),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText(m["settings.keyManagement.advanced"]()));
    fireEvent.click(
      screen.getByRole("button", { name: m["settings.keyManagement.publicKey.option"]() }),
    );

    expect(
      screen.getByText(m["settings.keyManagement.publicKey.warning.title"]()),
    ).toBeInTheDocument();
    const continueButton = screen.getByRole("button", { name: m["common.continue"]() });
    expect(continueButton).toBeDisabled();

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: m["settings.keyManagement.publicKey.warning.confirmations.generated"](),
      }),
    );
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: m["settings.keyManagement.publicKey.warning.confirmations.privateKey"](),
      }),
    );
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: m["settings.keyManagement.publicKey.warning.confirmations.authority"](),
      }),
    );

    expect(continueButton).not.toBeDisabled();
    fireEvent.click(continueButton);

    const textArea = screen.getByLabelText(
      m["settings.keyManagement.publicKey.input.label"](),
    ) as HTMLTextAreaElement;
    fireEvent.change(textArea, { target: { value: publicKeyInput } });

    expect(
      screen.getByText(m["settings.keyManagement.publicKey.input.valid"]()),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: m["settings.keyManagement.publicKey.input.submit"]() }),
    );

    expect(
      screen.getByText(m["settings.keyManagement.publicKey.summary.title"]()),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        `${encodeHex(publicKey).slice(0, 10)} ... ${encodeHex(publicKey).slice(-4)}`,
      ),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: m["settings.keyManagement.publicKey.summary.confirm"](),
      }),
    );

    await waitFor(() => {
      expect(updateKey).toHaveBeenCalledWith({
        action: {
          insert: {
            secp256k1: encodeBase64(publicKey),
          },
        },
        keyHash: createKeyHash(publicKey),
        sender: "0x73656e6465720000000000000000000000000000",
      });
    });
    expect(mocks.hideModal).toHaveBeenCalledOnce();
  });
});

function PublicKeyInputHarness() {
  const [publicKeyInput, setPublicKeyInput] = useState("");
  const [publicKey, setPublicKey] = useState<Uint8Array | null>(null);

  return (
    <AddKey.Context.Provider
      value={{
        state: {
          screen: "public-key-input",
          email: "",
          publicKeyInput,
          publicKey,
          isPending: false,
        },
        actions: {
          setScreen: mocks.setScreen,
          setEmail: vi.fn(),
          setPublicKeyInput,
          setPublicKey,
          linkEmailKey: async () => undefined,
          addKey: async () => undefined,
          addPublicKey: async () => undefined,
        },
      }}
    >
      <AddKey.PublicKeyInput />
    </AddKey.Context.Provider>
  );
}

function SubmitPublicKey({
  publicKey,
  label = "Submit duplicate public key",
}: {
  publicKey: Uint8Array;
  label?: string;
}) {
  const {
    actions: { addPublicKey },
  } = use(AddKey.Context);

  return (
    <button type="button" onClick={() => addPublicKey(publicKey).catch(() => undefined)}>
      {label}
    </button>
  );
}

function SubmitConnectorKey({ connectorId, label }: { connectorId: string; label: string }) {
  const {
    actions: { addKey },
  } = use(AddKey.Context);

  return (
    <button type="button" onClick={() => addKey(connectorId).catch(() => undefined)}>
      {label}
    </button>
  );
}

function SubmitEmailKey() {
  const {
    actions: { linkEmailKey },
  } = use(AddKey.Context);

  return (
    <button type="button" onClick={() => linkEmailKey().catch(() => undefined)}>
      Submit email key
    </button>
  );
}
