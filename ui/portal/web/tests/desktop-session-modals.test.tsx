import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitQRCodeReaderFactory,
  setAppletsKitUseApp,
} from "./mocks/applets-kit";
import { QRConnect } from "../src/components/modals/QRConnect";
import { SignWithDesktop } from "../src/components/modals/SignWithDesktop";
import { SignWithDesktopFromNativeCamera } from "../src/components/modals/SignWithDesktopFromNativeCamera";
import { WS_URI } from "../constants.config";

const desktopSessionMocks = vi.hoisted(() => ({
  captureException: vi.fn(),
  createSession: vi.fn(),
  createMessageExchanger: vi.fn(),
  hasConnectorClient: true,
  hideModal: vi.fn(),
  lastDesktopSigninOptions: undefined as
    | {
        mutation: {
          onSuccess?: () => void;
        };
        toast: {
          error: () => void;
        };
        url: string;
      }
    | undefined,
  qrScanner: undefined as ((socketId: string) => void) | undefined,
  qrCodeInstances: [] as Array<{
    append: ReturnType<typeof vi.fn>;
    options: unknown;
    update: ReturnType<typeof vi.fn>;
  }>,
  sendMessage: vi.fn(),
  subscribe: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  unsubscribe: vi.fn(),
  useSigninWithDesktop: vi.fn(),
}));

type Message = {
  id: string;
  message: unknown;
  type: string;
};

vi.mock("@sentry/react", () => ({
  captureException: desktopSessionMocks.captureException,
}));

vi.mock("qr-code-styling", () => ({
  default: vi.fn((options: unknown) => {
    const instance = {
      append: vi.fn(),
      options,
      update: vi.fn(),
    };

    desktopSessionMocks.qrCodeInstances.push(instance);

    return instance;
  }),
}));

vi.mock("@left-curve/store", () => ({
  MessageExchanger: {
    create: desktopSessionMocks.createMessageExchanger,
  },
  useAccount: () => ({
    userIndex: 42,
  }),
  useConnectorClient: () => ({
    data: desktopSessionMocks.hasConnectorClient
      ? {
          createSession: desktopSessionMocks.createSession,
        }
      : undefined,
  }),
  useSigninWithDesktop: desktopSessionMocks.useSigninWithDesktop,
}));

function createMessageExchangerFixture() {
  let listener: ((message: Message) => Promise<void>) | undefined;

  const exchanger = {
    emit: async (message: Message) => listener?.(message),
    getSocketId: () => "socket-qr-1",
    sendMessage: desktopSessionMocks.sendMessage,
    subscribe: desktopSessionMocks.subscribe.mockImplementation((callback) => {
      listener = callback;
      return desktopSessionMocks.unsubscribe;
    }),
  };

  return exchanger;
}

describe("desktop session modals", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseApp({
      hideModal: desktopSessionMocks.hideModal,
      toast: {
        error: desktopSessionMocks.toastError,
        success: desktopSessionMocks.toastSuccess,
      },
    });
    setAppletsKitQRCodeReaderFactory(({ onScan }) => {
      desktopSessionMocks.qrScanner = onScan;
      return <div data-testid="qr-code-reader" />;
    });
    desktopSessionMocks.hasConnectorClient = true;
    desktopSessionMocks.qrCodeInstances = [];
    desktopSessionMocks.createSession.mockResolvedValue({
      sessionInfo: {
        keyHash: "session-key",
      },
    });
    desktopSessionMocks.useSigninWithDesktop.mockImplementation((options) => {
      desktopSessionMocks.lastDesktopSigninOptions = options;
      return {
        isPending: false,
        mutateAsync: vi.fn().mockImplementation(async ({ socketId }: { socketId: string }) => {
          options.mutation.onSuccess?.();
          return { socketId };
        }),
      };
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    desktopSessionMocks.lastDesktopSigninOptions = undefined;
    desktopSessionMocks.qrCodeInstances = [];
    desktopSessionMocks.qrScanner = undefined;
  });

  it("renders a QR socket URL and creates a mobile session from exchanger messages", async () => {
    const exchanger = createMessageExchangerFixture();
    desktopSessionMocks.createMessageExchanger.mockResolvedValue(exchanger);
    const { unmount } = render(<QRConnect />);
    const expectedQrData = `${document.location.origin}/?socketId=socket-qr-1`;

    expect(screen.getByText(m["modals.qrconnect.title"]())).toBeInTheDocument();
    expect(screen.getByText(m["modals.qrconnect.description"]())).toBeInTheDocument();
    await waitFor(() => {
      const qrCode = desktopSessionMocks.qrCodeInstances.at(-1);

      expect(qrCode?.append).toHaveBeenCalledWith(expect.any(HTMLDivElement));
      expect(qrCode?.update).toHaveBeenCalledWith({ data: expectedQrData });
    });
    expect(desktopSessionMocks.subscribe).toHaveBeenCalledOnce();

    await act(async () => {
      await exchanger.emit({
        id: "message-1",
        message: {
          expireAt: 1760000000,
          publicKey: "AQIDBA==",
        },
        type: "create-session",
      });
    });

    expect(desktopSessionMocks.createSession).toHaveBeenCalledWith({
      expireAt: 1760000000,
      pubKey: Uint8Array.from([1, 2, 3, 4]),
    });
    expect(desktopSessionMocks.sendMessage).toHaveBeenCalledWith({
      id: "message-1",
      message: {
        data: {
          sessionInfo: {
            keyHash: "session-key",
          },
          userIndex: 42,
        },
      },
    });
    expect(desktopSessionMocks.toastSuccess).toHaveBeenCalledWith({
      title: "Connection established",
      description: null,
    });
    expect(desktopSessionMocks.hideModal).toHaveBeenCalledOnce();

    unmount();
    expect(desktopSessionMocks.unsubscribe).toHaveBeenCalledOnce();
  });

  it("returns mobile session errors through the exchanger and app toast", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const exchanger = createMessageExchangerFixture();
    const error = new Error("rejected by wallet");
    desktopSessionMocks.createSession.mockRejectedValue(error);
    desktopSessionMocks.createMessageExchanger.mockResolvedValue(exchanger);

    render(<QRConnect />);
    await waitFor(() => expect(desktopSessionMocks.subscribe).toHaveBeenCalledOnce());

    await act(async () => {
      await exchanger.emit({
        id: "message-2",
        message: {
          expireAt: 1760000000,
          publicKey: "AQIDBA==",
        },
        type: "create-session",
      });
    });

    expect(desktopSessionMocks.captureException).toHaveBeenCalledWith(error);
    expect(desktopSessionMocks.toastError).toHaveBeenCalledWith({
      title: m["common.error"](),
      description: m["signin.errors.mobileSessionAborted"](),
    });
    expect(desktopSessionMocks.sendMessage).toHaveBeenCalledWith({
      id: "message-2",
      message: {
        error: "rejected by wallet",
      },
    });
    expect(desktopSessionMocks.hideModal).toHaveBeenCalledOnce();

    consoleError.mockRestore();
  });

  it("ignores mobile session requests when no connector client is available", async () => {
    const exchanger = createMessageExchangerFixture();
    desktopSessionMocks.hasConnectorClient = false;
    desktopSessionMocks.createMessageExchanger.mockResolvedValue(exchanger);

    render(<QRConnect />);
    await waitFor(() => expect(desktopSessionMocks.subscribe).toHaveBeenCalledOnce());

    await act(async () => {
      await exchanger.emit({
        id: "message-without-client",
        message: {
          expireAt: 1760000000,
          publicKey: "AQIDBA==",
        },
        type: "create-session",
      });
    });

    expect(desktopSessionMocks.createSession).not.toHaveBeenCalled();
    expect(desktopSessionMocks.sendMessage).not.toHaveBeenCalled();
    expect(desktopSessionMocks.toastSuccess).not.toHaveBeenCalled();
    expect(desktopSessionMocks.toastError).not.toHaveBeenCalled();
    expect(desktopSessionMocks.hideModal).not.toHaveBeenCalled();
  });

  it("scans desktop QR codes and closes on successful desktop sign-in", async () => {
    const mutateAsync = vi.fn().mockImplementation(async ({ socketId }: { socketId: string }) => {
      desktopSessionMocks.lastDesktopSigninOptions?.mutation.onSuccess?.();
      return { socketId };
    });
    desktopSessionMocks.useSigninWithDesktop.mockImplementation((options) => {
      desktopSessionMocks.lastDesktopSigninOptions = options;
      return {
        isPending: false,
        mutateAsync,
      };
    });

    render(<SignWithDesktop />);

    expect(desktopSessionMocks.useSigninWithDesktop).toHaveBeenCalledWith({
      url: WS_URI,
      toast: {
        error: expect.any(Function),
      },
      mutation: {
        onSuccess: expect.any(Function),
      },
    });
    expect(screen.getByTestId("qr-code-reader")).toBeInTheDocument();

    await act(async () => {
      desktopSessionMocks.qrScanner?.("desktop-socket-1");
    });

    expect(mutateAsync).toHaveBeenCalledWith({ socketId: "desktop-socket-1" });
    expect(desktopSessionMocks.hideModal).toHaveBeenCalledOnce();

    desktopSessionMocks.lastDesktopSigninOptions?.toast.error();
    expect(desktopSessionMocks.toastError).toHaveBeenCalledWith({
      title: m["common.error"](),
      description: m["signin.errors.failedSignInWithDesktop"](),
    });
  });

  it("shows pending desktop authorization and signs in from native-camera socket IDs", async () => {
    desktopSessionMocks.useSigninWithDesktop.mockImplementationOnce((options) => {
      desktopSessionMocks.lastDesktopSigninOptions = options;
      return {
        isPending: true,
        mutateAsync: vi.fn(),
      };
    });

    render(<SignWithDesktop />);

    expect(document.querySelector(".animate-spinner-ease-spin")?.parentElement).toHaveClass(
      "w-8",
      "h-8",
    );
    expect(screen.getByText(m["signin.authorizeInDesktop"]())).toBeInTheDocument();

    cleanup();
    const mutateAsync = vi.fn().mockResolvedValue(undefined);
    desktopSessionMocks.useSigninWithDesktop.mockImplementation((options) => {
      desktopSessionMocks.lastDesktopSigninOptions = options;
      return {
        isPending: false,
        mutateAsync,
      };
    });

    render(<SignWithDesktopFromNativeCamera socketId="native-camera-socket" />);

    expect(mutateAsync).toHaveBeenCalledWith({ socketId: "native-camera-socket" });
    expect(document.querySelector(".animate-spinner-ease-spin")).toHaveClass(
      "border-b-primitives-blue-light-300",
    );
    expect(screen.getByText(m["signin.authorizeInDesktop"]())).toBeInTheDocument();
  });
});
