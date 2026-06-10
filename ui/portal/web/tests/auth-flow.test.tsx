import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetAppletsKitMocks,
  setAppletsKitUseAppFactory,
  setAppletsKitUseMediaQueryFactory,
} from "./mocks/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Modals } from "@left-curve/applets-kit";

import { DEFAULT_SESSION_EXPIRATION } from "../constants.config";
import { AuthFlow } from "../src/components/auth/AuthFlow";

const authFlowMocks = vi.hoisted(() => ({
  authenticate: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  authenticateDebug: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  changeSettings: vi.fn(),
  connectors: [] as Array<{ id: string; name?: string; type: string }>,
  createAccount: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  createNewWithExistingKey: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  initialEmail: "",
  initialReferrer: undefined as number | undefined,
  initialScreen: "options",
  isMd: true,
  passkeyCreate: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  passkeyLogin: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  selectAccount: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  setEmail: vi.fn(),
  setReferrer: vi.fn(),
  setScreen: vi.fn(),
  showModal: vi.fn(),
  toastError: vi.fn(),
  useAuthState: vi.fn(),
  users: [
    { index: 3, name: "alice" },
    { index: 7, name: "bob" },
  ],
}));

vi.mock("@left-curve/store", async () => {
  const React = await import("react");

  return {
    useAuthState: (options: unknown) => {
      authFlowMocks.useAuthState(options);

      const [screen, setScreenState] = React.useState(authFlowMocks.initialScreen);
      const [email, setEmailState] = React.useState(authFlowMocks.initialEmail);
      const [referrer, setReferrerState] = React.useState(authFlowMocks.initialReferrer);

      return {
        authenticate: authFlowMocks.authenticate,
        authenticateDebug: authFlowMocks.authenticateDebug,
        createAccount: authFlowMocks.createAccount,
        createNewWithExistingKey: authFlowMocks.createNewWithExistingKey,
        email,
        identifier: "very-long-authentication-identifier@example.com",
        passkeyCreate: authFlowMocks.passkeyCreate,
        passkeyLogin: authFlowMocks.passkeyLogin,
        referrer,
        screen,
        selectAccount: authFlowMocks.selectAccount,
        setEmail: (nextEmail: string) => {
          authFlowMocks.setEmail(nextEmail);
          setEmailState(nextEmail);
        },
        setReferrer: (nextReferrer?: number) => {
          authFlowMocks.setReferrer(nextReferrer);
          setReferrerState(nextReferrer);
        },
        setScreen: (nextScreen: string) => {
          authFlowMocks.setScreen(nextScreen);
          setScreenState(nextScreen);
        },
        users: authFlowMocks.users,
      };
    },
    useConnectors: () => authFlowMocks.connectors,
  };
});

vi.mock("../src/components/foundation/DangoLogo", () => ({
  DangoLogo: () => <div data-testid="dango-logo" />,
}));

vi.mock("../src/components/auth/EmailCredential", () => ({
  EmailCredential: {
    Email: ({
      onChange,
      onSubmit,
      value,
    }: {
      onChange: (value: string) => void;
      onSubmit: () => void;
      value: string;
    }) => (
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <input
          aria-label="email"
          onChange={(event) => onChange(event.target.value)}
          value={value}
        />
        <button type="submit">continue with email</button>
      </form>
    ),
    OTP: ({
      email,
      goBack,
      onSuccess,
    }: {
      email: string;
      goBack: () => void;
      onSuccess: () => void;
    }) => (
      <div>
        <p>{`otp:${email}`}</p>
        <button onClick={onSuccess} type="button">
          submit otp
        </button>
        <button onClick={goBack} type="button">
          back from otp
        </button>
      </div>
    ),
  },
}));

vi.mock("../src/components/auth/SocialCredential", () => ({
  SocialCredential: ({ onAuth }: { onAuth: () => void }) => (
    <button onClick={onAuth} type="button">
      social auth
    </button>
  ),
}));

vi.mock("../src/components/auth/AuthOptions", () => ({
  AuthOptions: ({ action, isPending }: { action: (id: string) => void; isPending: boolean }) => (
    <div>
      <button disabled={isPending} onClick={() => action("wallet-a")} type="button">
        Wallet A
      </button>
      <button disabled={isPending} onClick={() => action("wallet-b")} type="button">
        Wallet B
      </button>
    </div>
  ),
}));

vi.mock("../src/components/auth/UsernamesList", () => ({
  UsernamesList: ({
    onUserSelection,
    users,
  }: {
    onUserSelection: (userIndex: number) => void;
    users: Array<{ index: number; name: string }>;
  }) => (
    <div>
      {users.map((user) => (
        <button key={user.index} onClick={() => onUserSelection(user.index)} type="button">
          {user.name}
        </button>
      ))}
    </div>
  ),
}));

function renderAuthFlow(referrer?: number) {
  const onFinish = vi.fn();
  const rendered = render(<AuthFlow onFinish={onFinish} referrer={referrer} />);

  return {
    onFinish,
    ...rendered,
  };
}

function latestAuthOptions() {
  return authFlowMocks.useAuthState.mock.calls.at(-1)?.[0] as {
    expiration: number;
    onError: (error: unknown) => void;
    onSuccess: () => void;
    referrer?: number;
    session: boolean;
  };
}

describe("AuthFlow", () => {
  beforeEach(() => {
    window.history.pushState({}, "", "/");
    authFlowMocks.authenticate.isPending = false;
    authFlowMocks.authenticate.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.authenticateDebug.isPending = false;
    authFlowMocks.authenticateDebug.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.connectors = [{ id: "wallet-a", name: "Wallet A", type: "wallet" }];
    authFlowMocks.createAccount.isPending = false;
    authFlowMocks.createAccount.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.createNewWithExistingKey.isPending = false;
    authFlowMocks.createNewWithExistingKey.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.initialEmail = "";
    authFlowMocks.initialReferrer = undefined;
    authFlowMocks.initialScreen = "options";
    authFlowMocks.isMd = true;
    authFlowMocks.passkeyCreate.isPending = false;
    authFlowMocks.passkeyCreate.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.passkeyLogin.isPending = false;
    authFlowMocks.passkeyLogin.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.selectAccount.isPending = false;
    authFlowMocks.selectAccount.mutateAsync.mockResolvedValue(undefined);
    authFlowMocks.users = [
      { index: 3, name: "alice" },
      { index: 7, name: "bob" },
    ];
    resetAppletsKitMocks();
    setAppletsKitUseAppFactory(() => ({
      changeSettings: authFlowMocks.changeSettings,
      settings: {
        useSessionKey: true,
      },
      showModal: authFlowMocks.showModal,
      toast: {
        error: authFlowMocks.toastError,
      },
    }));
    setAppletsKitUseMediaQueryFactory(() => ({
      isMd: authFlowMocks.isMd,
    }));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    window.history.pushState({}, "", "/");
  });

  it("configures the auth hook from app settings and URL referrer, then exposes success and error handlers", () => {
    window.history.pushState({}, "", "/?ref=77");
    const { onFinish } = renderAuthFlow();

    const options = latestAuthOptions();

    expect(options).toEqual(
      expect.objectContaining({
        expiration: DEFAULT_SESSION_EXPIRATION,
        referrer: 77,
        session: true,
      }),
    );

    options.onSuccess();
    expect(onFinish).toHaveBeenCalledOnce();

    options.onError(new Error("wallet rejected"));
    expect(authFlowMocks.toastError).toHaveBeenCalledWith({
      description: "wallet rejected",
      title: m["common.error"](),
    });

    cleanup();
    vi.clearAllMocks();
    renderAuthFlow(9);

    expect(latestAuthOptions()).toEqual(
      expect.objectContaining({
        referrer: 9,
      }),
    );
  });

  it("moves from welcome email entry to OTP auth and back to clean email state", () => {
    renderAuthFlow();

    fireEvent.change(screen.getByLabelText("email"), {
      target: {
        value: "alice@example.com",
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "continue with email" }));

    expect(screen.getByText("otp:alice@example.com")).toBeInTheDocument();
    expect(authFlowMocks.setScreen).toHaveBeenCalledWith("email");

    fireEvent.click(screen.getByRole("button", { name: "submit otp" }));
    expect(authFlowMocks.authenticate.mutateAsync).toHaveBeenCalledWith("privy");

    fireEvent.click(screen.getByRole("button", { name: "back from otp" }));

    expect(authFlowMocks.setScreen).toHaveBeenCalledWith("options");
    expect(authFlowMocks.setEmail).toHaveBeenCalledWith("");
    expect(screen.getByLabelText("email")).toHaveValue("");
  });

  it("runs welcome-screen provider actions including debug and desktop sign-in", () => {
    window.history.pushState({}, "", "/?debugAs=12");
    authFlowMocks.isMd = false;

    renderAuthFlow();

    fireEvent.click(screen.getByRole("button", { name: "social auth" }));
    expect(authFlowMocks.authenticate.mutateAsync).toHaveBeenCalledWith("privy");

    fireEvent.click(screen.getByRole("button", { name: m["common.connectWithPasskey"]() }));
    expect(authFlowMocks.authenticate.mutateAsync).toHaveBeenCalledWith("passkey");

    fireEvent.click(screen.getByRole("button", { name: "Debug as user #12" }));
    expect(authFlowMocks.authenticateDebug.mutateAsync).toHaveBeenCalledWith(12);

    fireEvent.click(screen.getByRole("button", { name: m["common.signinWithDesktop"]() }));
    expect(authFlowMocks.showModal).toHaveBeenCalledWith(Modals.SignWithDesktop);
  });

  it("preserves zero-valued backend user indexes from auth query params", () => {
    window.history.pushState({}, "", "/?ref=0&debugAs=0");

    renderAuthFlow();

    expect(latestAuthOptions()).toEqual(
      expect.objectContaining({
        referrer: 0,
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "Debug as user #0" }));
    expect(authFlowMocks.authenticateDebug.mutateAsync).toHaveBeenCalledWith(0);

    cleanup();
    vi.clearAllMocks();
    window.history.pushState({}, "", "/?ref=0");
    authFlowMocks.initialReferrer = 0;
    authFlowMocks.initialScreen = "create-account";

    renderAuthFlow();

    const referralInput = screen.getByDisplayValue("0");
    expect(referralInput).toBeDisabled();
  });

  it("routes wallet and passkey choices through the auth state mutations", () => {
    authFlowMocks.initialScreen = "wallets";
    renderAuthFlow();

    expect(screen.getByText(m["signin.connectWalletToContinue"]())).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Wallet A" }));
    expect(authFlowMocks.authenticate.mutateAsync).toHaveBeenCalledWith("wallet-a");

    authFlowMocks.initialScreen = "passkey-choice";
    cleanup();
    renderAuthFlow();

    fireEvent.click(screen.getByRole("button", { name: m["auth.passkeyChoice.createNew"]() }));
    expect(authFlowMocks.passkeyCreate.mutateAsync).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: m["auth.passkeyChoice.useExisting"]() }));
    expect(authFlowMocks.passkeyLogin.mutateAsync).toHaveBeenCalledOnce();
  });

  it("locks URL referrers on account creation but allows account-picker choices", () => {
    window.history.pushState({}, "", "/?ref=42");
    authFlowMocks.initialReferrer = 42;
    authFlowMocks.initialScreen = "create-account";

    renderAuthFlow();

    const referralInput = screen.getByDisplayValue("42");
    expect(referralInput).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: m["common.continue"]() }));
    expect(authFlowMocks.createAccount.mutateAsync).toHaveBeenCalledOnce();

    cleanup();
    vi.clearAllMocks();
    window.history.pushState({}, "", "/");
    authFlowMocks.initialScreen = "account-picker";

    renderAuthFlow();

    fireEvent.click(screen.getByRole("button", { name: "bob" }));
    expect(authFlowMocks.selectAccount.mutateAsync).toHaveBeenCalledWith(7);

    fireEvent.click(screen.getByRole("button", { name: m["common.createNewUser"]() }));
    expect(authFlowMocks.createNewWithExistingKey.mutateAsync).toHaveBeenCalledOnce();
  });

  it("preserves backend user index zero when selecting an existing account", () => {
    authFlowMocks.initialScreen = "account-picker";
    authFlowMocks.users = [
      { index: 0, name: "genesis" },
      { index: 7, name: "bob" },
    ];

    renderAuthFlow();

    fireEvent.click(screen.getByRole("button", { name: "genesis" }));

    expect(authFlowMocks.selectAccount.mutateAsync).toHaveBeenCalledWith(0);
  });
});
