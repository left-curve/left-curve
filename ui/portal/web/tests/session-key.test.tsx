import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSessionKey } from "../../../store/src/hooks/useSessionKey";
import { createQueryClientWrapper } from "./utils/query-client";

const hookMocks = vi.hoisted(() => ({
  createSessionSigner: vi.fn(),
  createSignerClient: vi.fn(),
  makeKeyPair: vi.fn(),
  useAccount: vi.fn(),
  useConfig: vi.fn(),
  useStorage: vi.fn(),
}));

vi.mock("@left-curve/sdk", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    createSessionSigner: hookMocks.createSessionSigner,
    createSignerClient: hookMocks.createSignerClient,
  };
});

vi.mock("@left-curve/crypto", () => ({
  Secp256k1: {
    makeKeyPair: hookMocks.makeKeyPair,
  },
}));

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: hookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: hookMocks.useConfig,
}));

vi.mock("../../../store/src/hooks/useStorage.js", async () => {
  const React = await import("react");

  return {
    useStorage: hookMocks.useStorage.mockImplementation(
      (_key: string, options: { initialValue?: unknown } = {}) =>
        React.useState(options.initialValue ?? null),
    ),
  };
});

const storedSession = {
  authorization: {
    keyHash: "stored-key-hash",
    signature: "stored-signature",
  },
  keyHash: "stored-key-hash",
  privateKey: new Uint8Array([9, 9, 9]),
  publicKey: new Uint8Array([1, 2, 3]),
  sessionInfo: {
    chainId: "dango-dev-1",
    expireAt: "1700003600",
    sessionKey: "stored-session-key",
  },
};

describe("useSessionKey", () => {
  beforeEach(() => {
    hookMocks.useConfig.mockReturnValue({
      _internal: {
        transport: "test-transport",
      },
      chain: {
        id: "dango-dev-1",
      },
    });
    hookMocks.useAccount.mockReturnValue({
      connector: undefined,
      username: "alice",
    });
    hookMocks.createSessionSigner.mockReturnValue("session-signer");
    hookMocks.createSignerClient.mockResolvedValue("session-client");
    hookMocks.useStorage.mockClear();
    hookMocks.makeKeyPair.mockReturnValue({
      privateKey: new Uint8Array([7, 8, 9]),
      getPublicKey: vi.fn(() => new Uint8Array([1, 2, 3])),
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("creates a signer client from the stored session when a username is available", async () => {
    const { result } = renderHook(() => useSessionKey({ session: storedSession }), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.client).toBe("session-client"));

    expect(hookMocks.createSessionSigner).toHaveBeenCalledWith(storedSession);
    expect(hookMocks.createSignerClient).toHaveBeenCalledWith({
      chain: { id: "dango-dev-1" },
      sessionKey: "stored-session-key",
      signer: "session-signer",
      transport: "test-transport",
      type: "session",
      username: "alice",
    });
  });

  it("keeps stored sessions idle until a username is available", () => {
    hookMocks.useAccount.mockReturnValue({
      connector: undefined,
      username: undefined,
    });

    const { result } = renderHook(() => useSessionKey({ session: storedSession }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current.session).toEqual(storedSession);
    expect(result.current.client).toBeUndefined();
    expect(hookMocks.createSessionSigner).not.toHaveBeenCalled();
    expect(hookMocks.createSignerClient).not.toHaveBeenCalled();
  });

  it("uses synchronized session storage with the provided initial session", () => {
    renderHook(() => useSessionKey({ session: storedSession }), {
      wrapper: createQueryClientWrapper(),
    });

    expect(hookMocks.useStorage).toHaveBeenCalledWith(
      "session_key",
      expect.objectContaining({
        initialValue: storedSession,
        storage: expect.any(Object),
        sync: true,
        version: 1.2,
      }),
    );
    const [, options] = hookMocks.useStorage.mock.calls.at(-1) ?? [];
    expect(options.migrations["*"]()).toBeNull();
  });

  it("signs and stores a generated session key by default", async () => {
    const connector = {
      signArbitrary: vi.fn().mockResolvedValue({
        credential: {
          standard: {
            keyHash: "generated-key-hash",
            signature: "generated-signature",
          },
        },
      }),
    };

    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    let session: Awaited<ReturnType<typeof result.current.createSessionKey>>;
    await act(async () => {
      session = await result.current.createSessionKey({
        connector,
        expireAt: 1_700_003_600_000,
      });
    });

    expect(connector.signArbitrary).toHaveBeenCalledWith({
      primaryType: "Message",
      message: {
        chainId: "dango-dev-1",
        expireAt: "1700003600",
        sessionKey: "AQID",
      },
      types: {
        Message: [
          { name: "chain_id", type: "string" },
          { name: "expire_at", type: "string" },
          { name: "session_key", type: "string" },
        ],
      },
    });
    expect(session!).toEqual({
      authorization: {
        keyHash: "generated-key-hash",
        signature: "generated-signature",
      },
      keyHash: "generated-key-hash",
      privateKey: new Uint8Array([7, 8, 9]),
      publicKey: new Uint8Array([1, 2, 3]),
      sessionInfo: {
        chainId: "dango-dev-1",
        expireAt: "1700003600",
        sessionKey: "AQID",
      },
    });
    expect(result.current.session).toEqual(session!);
  });

  it("can create a session key without loading it into storage", async () => {
    const connector = {
      signArbitrary: vi.fn().mockResolvedValue({
        credential: {
          standard: {
            keyHash: "detached-key-hash",
            signature: "detached-signature",
          },
        },
      }),
    };

    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.createSessionKey(
        {
          connector,
          expireAt: 1_700_003_600_000,
        },
        { setSession: false },
      );
    });

    expect(result.current.session).toBeNull();
  });

  it("uses the active account connector when creating a session key", async () => {
    const connector = {
      signArbitrary: vi.fn().mockResolvedValue({
        credential: {
          standard: {
            keyHash: "account-connector-key-hash",
            signature: "account-connector-signature",
          },
        },
      }),
    };
    hookMocks.useAccount.mockReturnValue({
      connector,
      username: "alice",
    });

    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    await act(async () => {
      await result.current.createSessionKey({
        expireAt: 1_700_003_600_000,
      });
    });

    expect(connector.signArbitrary).toHaveBeenCalledWith({
      primaryType: "Message",
      message: {
        chainId: "dango-dev-1",
        expireAt: "1700003600",
        sessionKey: "AQID",
      },
      types: {
        Message: [
          { name: "chain_id", type: "string" },
          { name: "expire_at", type: "string" },
          { name: "session_key", type: "string" },
        ],
      },
    });
    expect(result.current.session?.authorization).toEqual({
      keyHash: "account-connector-key-hash",
      signature: "account-connector-signature",
    });
  });

  it("does not generate or store a session key when no connector is available", async () => {
    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.createSessionKey({
          expireAt: 1_700_003_600_000,
        });
      }),
    ).rejects.toThrow("connector not found");
    expect(hookMocks.makeKeyPair).not.toHaveBeenCalled();
    expect(result.current.session).toBeNull();
  });

  it("rejects unsupported credentials without storing a session key", async () => {
    const connector = {
      signArbitrary: vi.fn().mockResolvedValue({
        credential: {
          webauthn: {
            keyHash: "unsupported-key-hash",
            signature: "unsupported-signature",
          },
        },
      }),
    };

    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.createSessionKey({
          connector,
          expireAt: 1_700_003_600_000,
        });
      }),
    ).rejects.toThrow("unsupported credential type");
    expect(result.current.session).toBeNull();
  });

  it("does not store a generated session key when connector signing fails", async () => {
    const signingError = new Error("session signing rejected");
    const connector = {
      signArbitrary: vi.fn().mockRejectedValue(signingError),
    };

    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.createSessionKey({
          connector,
          expireAt: 1_700_003_600_000,
        });
      }),
    ).rejects.toThrow("session signing rejected");

    expect(hookMocks.makeKeyPair).toHaveBeenCalledOnce();
    expect(connector.signArbitrary).toHaveBeenCalledWith({
      primaryType: "Message",
      message: {
        chainId: "dango-dev-1",
        expireAt: "1700003600",
        sessionKey: "AQID",
      },
      types: {
        Message: [
          { name: "chain_id", type: "string" },
          { name: "expire_at", type: "string" },
          { name: "session_key", type: "string" },
        ],
      },
    });
    expect(result.current.session).toBeNull();
  });

  it("clears the stored session key", async () => {
    const { result } = renderHook(() => useSessionKey({ session: storedSession }), {
      wrapper: createQueryClientWrapper(),
    });

    act(() => result.current.deleteSessionKey());

    expect(result.current.session).toBeNull();
  });

  it("creates a signer client after receiving a synchronized session", async () => {
    let setSession: ((session: typeof storedSession) => void) | undefined;
    hookMocks.useStorage.mockImplementationOnce((_key: string) => {
      const state = useState(null);
      setSession = state[1] as (session: typeof storedSession) => void;
      return state;
    });
    const { result } = renderHook(() => useSessionKey(), {
      wrapper: createQueryClientWrapper(),
    });

    expect(hookMocks.createSessionSigner).not.toHaveBeenCalled();
    expect(hookMocks.createSignerClient).not.toHaveBeenCalled();

    act(() => setSession?.(storedSession));

    await waitFor(() => expect(result.current.client).toBe("session-client"));

    expect(result.current.session).toEqual(storedSession);
    expect(hookMocks.createSessionSigner).toHaveBeenCalledWith(storedSession);
    expect(hookMocks.createSignerClient).toHaveBeenCalledWith({
      chain: { id: "dango-dev-1" },
      sessionKey: "stored-session-key",
      signer: "session-signer",
      transport: "test-transport",
      type: "session",
      username: "alice",
    });
  });
});
