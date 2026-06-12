import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { decodeBase64, decodeUtf8, deserializeJson, encodeBase64 } from "@left-curve/encoding";
import { useSigninWithDesktop } from "../../../store/src/hooks/useSigninWithDesktop";

type SubmitTxMutation = {
  mutationFn: (variables: { socketId: string }) => Promise<void>;
};

const hookMocks = vi.hoisted(() => ({
  createMessageExchanger: vi.fn(),
  createPeerConnection: vi.fn(),
  makeKeyPair: vi.fn(),
  sendMessage: vi.fn(),
  sessionConnect: vi.fn(),
  useChainId: vi.fn(),
  useConnectors: vi.fn(),
  useSubmitTx: vi.fn(),
}));

vi.mock("@left-curve/crypto", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    Secp256k1: {
      makeKeyPair: hookMocks.makeKeyPair,
    },
  };
});

vi.mock("../../../store/src/messageExchanger.js", () => ({
  MessageExchanger: {
    create: hookMocks.createMessageExchanger,
  },
}));

vi.mock("../../../store/src/hooks/useChainId.js", () => ({
  useChainId: hookMocks.useChainId,
}));

vi.mock("../../../store/src/hooks/useConnectors.js", () => ({
  useConnectors: hookMocks.useConnectors,
}));

vi.mock("../../../store/src/hooks/useSubmitTx.js", () => ({
  useSubmitTx: hookMocks.useSubmitTx,
}));

const publicKey = new Uint8Array([1, 2, 3]);
const privateKey = new Uint8Array([9, 8, 7]);
const expiresAt = 1_780_000_000_000;

function createSessionConnector() {
  return {
    connect: hookMocks.sessionConnect,
    id: "session",
    uid: "session-connector",
  };
}

function decodeChallenge(challenge: string) {
  return deserializeJson<{
    authorization: string;
    keyHash: string;
    privateKey: Uint8Array;
    publicKey: Uint8Array;
    sessionInfo: {
      expireAt: string;
      sessionKey: string;
    };
  }>(decodeUtf8(decodeBase64(challenge)));
}

describe("useSigninWithDesktop", () => {
  beforeEach(() => {
    hookMocks.useChainId.mockReturnValue("dango-dev-1");
    hookMocks.useConnectors.mockReturnValue([createSessionConnector()]);
    hookMocks.useSubmitTx.mockImplementation(({ mutation }: { mutation: SubmitTxMutation }) => ({
      isPending: false,
      mutateAsync: (variables: { socketId: string }) => mutation.mutationFn(variables),
    }));
    hookMocks.createMessageExchanger.mockResolvedValue({
      createPeerConnection: hookMocks.createPeerConnection,
      sendMessage: hookMocks.sendMessage,
    });
    hookMocks.makeKeyPair.mockReturnValue({
      getPublicKey: vi.fn(() => publicKey),
      privateKey,
    });
    hookMocks.sendMessage.mockResolvedValue({
      data: {
        authorization: "desktop-authorization",
        keyHash: "session-key-hash",
        sessionInfo: {
          expireAt: "1780003600",
          sessionKey: "session-public-key",
        },
        userIndex: 7,
      },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("creates a desktop session request and connects the session connector with an encoded challenge", async () => {
    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await act(async () => {
      await result.current.mutateAsync({
        socketId: "socket-1",
      });
    });

    expect(hookMocks.createMessageExchanger).toHaveBeenCalledWith("wss://desktop.example");
    expect(hookMocks.createPeerConnection).toHaveBeenCalledWith("socket-1");
    expect(hookMocks.sendMessage).toHaveBeenCalledWith({
      type: "create-session",
      message: {
        expireAt: expiresAt,
        publicKey: encodeBase64(publicKey),
      },
    });

    expect(hookMocks.sessionConnect).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      challenge: expect.any(String),
      userIndex: 7,
    });

    const challenge = decodeChallenge(hookMocks.sessionConnect.mock.calls[0][0].challenge);
    expect(challenge).toEqual({
      authorization: "desktop-authorization",
      keyHash: "session-key-hash",
      privateKey,
      publicKey,
      sessionInfo: {
        expireAt: "1780003600",
        sessionKey: "session-public-key",
      },
    });
  });

  it("uses a one-day desktop session expiration when no explicit expiry is supplied", async () => {
    const dateNow = vi.spyOn(Date, "now").mockReturnValue(1_700_000_000_000);

    try {
      const { result } = renderHook(() =>
        useSigninWithDesktop({
          url: "wss://desktop.example",
        }),
      );

      await act(async () => {
        await result.current.mutateAsync({
          socketId: "socket-1",
        });
      });

      expect(hookMocks.sendMessage).toHaveBeenCalledWith({
        type: "create-session",
        message: {
          expireAt: 1_700_086_400_000,
          publicKey: encodeBase64(publicKey),
        },
      });
      expect(hookMocks.sessionConnect).toHaveBeenCalledWith(
        expect.objectContaining({
          chainId: "dango-dev-1",
          userIndex: 7,
        }),
      );
    } finally {
      dateNow.mockRestore();
    }
  });

  it("uses explicitly supplied connectors instead of the fallback connector list", async () => {
    const explicitSessionConnector = {
      connect: vi.fn().mockResolvedValue(undefined),
      id: "session",
      uid: "explicit-session",
    };

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        connectors: [explicitSessionConnector as ReturnType<typeof createSessionConnector>],
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await act(async () => {
      await result.current.mutateAsync({
        socketId: "socket-1",
      });
    });

    expect(hookMocks.useConnectors).toHaveBeenCalled();
    expect(explicitSessionConnector.connect).toHaveBeenCalledWith(
      expect.objectContaining({
        chainId: "dango-dev-1",
        userIndex: 7,
      }),
    );
    expect(hookMocks.sessionConnect).not.toHaveBeenCalled();
  });

  it("connects desktop sessions for backend user index zero", async () => {
    hookMocks.sendMessage.mockResolvedValueOnce({
      data: {
        authorization: "desktop-authorization",
        keyHash: "session-key-hash",
        sessionInfo: {
          expireAt: "1780003600",
          sessionKey: "session-public-key",
        },
        userIndex: 0,
      },
    });

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await act(async () => {
      await result.current.mutateAsync({
        socketId: "socket-1",
      });
    });

    expect(hookMocks.sessionConnect).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      challenge: expect.any(String),
      userIndex: 0,
    });

    const challenge = decodeChallenge(hookMocks.sessionConnect.mock.calls[0][0].challenge);
    expect(challenge).toEqual({
      authorization: "desktop-authorization",
      keyHash: "session-key-hash",
      privateKey,
      publicKey,
      sessionInfo: {
        expireAt: "1780003600",
        sessionKey: "session-public-key",
      },
    });
  });

  it("fails before peer connection when no session connector is available", async () => {
    hookMocks.useConnectors.mockReturnValue([{ id: "wallet", uid: "wallet" }]);

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await expect(
      result.current.mutateAsync({
        socketId: "socket-1",
      }),
    ).rejects.toThrow("error: missing connector");

    expect(hookMocks.createMessageExchanger).toHaveBeenCalledWith("wss://desktop.example");
    expect(hookMocks.createPeerConnection).not.toHaveBeenCalled();
    expect(hookMocks.sessionConnect).not.toHaveBeenCalled();
  });

  it("does not open a peer connection or create keys when the desktop exchanger is unavailable", async () => {
    hookMocks.createMessageExchanger.mockRejectedValueOnce(new Error("desktop websocket failed"));

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await expect(
      result.current.mutateAsync({
        socketId: "socket-1",
      }),
    ).rejects.toThrow("desktop websocket failed");

    expect(hookMocks.createMessageExchanger).toHaveBeenCalledWith("wss://desktop.example");
    expect(hookMocks.createPeerConnection).not.toHaveBeenCalled();
    expect(hookMocks.makeKeyPair).not.toHaveBeenCalled();
    expect(hookMocks.sendMessage).not.toHaveBeenCalled();
    expect(hookMocks.sessionConnect).not.toHaveBeenCalled();
  });

  it("does not generate or request a session when the desktop peer connection fails", async () => {
    hookMocks.createPeerConnection.mockRejectedValueOnce(new Error("peer unavailable"));

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await expect(
      result.current.mutateAsync({
        socketId: "socket-1",
      }),
    ).rejects.toThrow("peer unavailable");

    expect(hookMocks.createPeerConnection).toHaveBeenCalledWith("socket-1");
    expect(hookMocks.makeKeyPair).not.toHaveBeenCalled();
    expect(hookMocks.sendMessage).not.toHaveBeenCalled();
    expect(hookMocks.sessionConnect).not.toHaveBeenCalled();
  });

  it("surfaces desktop session errors and does not connect", async () => {
    hookMocks.sendMessage.mockResolvedValue({
      error: new Error("desktop rejected"),
    });

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await expect(
      result.current.mutateAsync({
        socketId: "socket-1",
      }),
    ).rejects.toThrow("desktop rejected");

    expect(hookMocks.createPeerConnection).toHaveBeenCalledWith("socket-1");
    expect(hookMocks.sessionConnect).not.toHaveBeenCalled();
  });

  it("surfaces session connector failures after desktop authorization", async () => {
    hookMocks.sessionConnect.mockRejectedValueOnce(new Error("session connector rejected"));

    const { result } = renderHook(() =>
      useSigninWithDesktop({
        expiresAt,
        url: "wss://desktop.example",
      }),
    );

    await expect(
      result.current.mutateAsync({
        socketId: "socket-1",
      }),
    ).rejects.toThrow("session connector rejected");

    expect(hookMocks.createPeerConnection).toHaveBeenCalledWith("socket-1");
    expect(hookMocks.sendMessage).toHaveBeenCalledWith({
      type: "create-session",
      message: {
        expireAt: expiresAt,
        publicKey: encodeBase64(publicKey),
      },
    });
    expect(hookMocks.sessionConnect).toHaveBeenCalledWith({
      chainId: "dango-dev-1",
      challenge: expect.any(String),
      userIndex: 7,
    });
  });
});
