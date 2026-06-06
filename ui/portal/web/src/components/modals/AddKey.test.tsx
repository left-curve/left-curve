import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { use, useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Secp256k1 } from "@left-curve/crypto";
import { createKeyHash } from "@left-curve/sdk";

import { AddKey } from "./AddKey";

const mocks = vi.hoisted(() => ({
  setScreen: vi.fn(),
  toastError: vi.fn(),
  hideModal: vi.fn(),
  useAccount: vi.fn(),
  useConnectors: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
  useQuery: vi.fn(),
}));

vi.mock("@left-curve/applets-kit", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/applets-kit")>();

  return {
    ...actual,
    useApp: () => ({
      hideModal: mocks.hideModal,
      toast: {
        error: mocks.toastError,
        success: vi.fn(),
      },
    }),
  };
});

vi.mock("@left-curve/store", () => ({
  useAccount: mocks.useAccount,
  useConnectors: mocks.useConnectors,
  useSigningClient: mocks.useSigningClient,
  useSubmitTx: mocks.useSubmitTx,
}));

vi.mock("~/constants", () => ({
  PRIVY_ERRORS_MAPPING: {},
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
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: vi.fn(() => null),
        setItem: vi.fn(),
        removeItem: vi.fn(),
      },
    });
    mocks.useAccount.mockReturnValue({
      account: { address: "dango1sender" },
      username: "alice",
      userIndex: 0,
    });
    mocks.useConnectors.mockReturnValue([]);
    mocks.useSigningClient.mockReturnValue({
      data: {
        getUserKeys: vi.fn(),
        updateKey: vi.fn(),
      },
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
    render(<PublicKeyInputHarness />);

    const publicKeyInput = screen.getByLabelText("Public Key");
    fireEvent.change(publicKeyInput, { target: { value: "not a key" } });

    expect(screen.getByRole("button", { name: "Add key" })).toBeDisabled();
    expect(
      screen.getByText("This doesn't look like a valid secp256k1 key. Please check and try again."),
    ).toBeInTheDocument();

    fireEvent.submit(publicKeyInput.closest("form")!);

    expect(mocks.setScreen).not.toHaveBeenCalledWith("public-key-summary");
  });

  it("shows the duplicate-key toast when a public key already exists", async () => {
    const publicKey = new Secp256k1(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1),
    ).getPublicKey(true);

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
      expect(mocks.toastError).toHaveBeenCalledWith(
        expect.objectContaining({
          description: "Key already exists.",
        }),
      );
    });
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

function SubmitPublicKey({ publicKey }: { publicKey: Uint8Array }) {
  const {
    actions: { addPublicKey },
  } = use(AddKey.Context);

  return (
    <button type="button" onClick={() => addPublicKey(publicKey).catch(() => undefined)}>
      Submit duplicate public key
    </button>
  );
}
