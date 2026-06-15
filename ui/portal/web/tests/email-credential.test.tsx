import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useState } from "react";
import type React from "react";

import { EmailCredential } from "../src/components/auth/EmailCredential";
import { createTestQueryClient } from "./utils/query-client";

const emailCredentialMocks = vi.hoisted(() => ({
  loginWithCode: vi.fn(),
  sendCode: vi.fn(),
  useConnectors: vi.fn(),
  wait: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useConnectors: emailCredentialMocks.useConnectors,
}));

vi.mock("@left-curve/utils", async (importOriginal) => {
  const actual = await importOriginal<object>();

  return {
    ...actual,
    wait: emailCredentialMocks.wait,
  };
});

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  render(<QueryClientProvider client={queryClient}>{component}</QueryClientProvider>);
  return queryClient;
}

function getEmailInput() {
  const input = document.querySelector<HTMLInputElement>('input[name="email"]');

  if (!input) throw new Error("Expected email input to exist");

  return input;
}

function enterOtp(code: string) {
  fireEvent.change(screen.getByLabelText("Digit 1 of the verification code"), {
    target: { value: code },
  });
}

function EmailHarness({
  onChange,
  onSubmit,
}: {
  onChange?: (email: string) => void;
  onSubmit: () => void;
}) {
  const [email, setEmail] = useState("");

  return (
    <EmailCredential.Email
      value={email}
      onChange={(nextEmail) => {
        onChange?.(nextEmail);
        setEmail(nextEmail);
      }}
      onSubmit={onSubmit}
    />
  );
}

describe("EmailCredential", () => {
  beforeEach(() => {
    emailCredentialMocks.wait.mockResolvedValue(undefined);
    emailCredentialMocks.sendCode.mockResolvedValue(undefined);
    emailCredentialMocks.loginWithCode.mockResolvedValue(undefined);
    emailCredentialMocks.useConnectors.mockReturnValue([
      {
        id: "privy",
        privy: {
          auth: {
            email: {
              loginWithCode: emailCredentialMocks.loginWithCode,
              sendCode: emailCredentialMocks.sendCode,
            },
          },
        },
      },
    ]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("sends a trimmed email verification code and advances to the OTP step", async () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();
    renderWithQueryClient(<EmailHarness onChange={onChange} onSubmit={onSubmit} />);

    fireEvent.change(getEmailInput(), {
      target: { value: " alice@example.com " },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.submit"]() }));

    await waitFor(() => {
      expect(emailCredentialMocks.sendCode).toHaveBeenCalledWith("alice@example.com");
    });
    expect(onChange).toHaveBeenCalledWith(" alice@example.com ");
    expect(onSubmit).toHaveBeenCalledOnce();
  });

  it("blocks invalid email submissions before calling Privy", () => {
    renderWithQueryClient(<EmailHarness onSubmit={vi.fn()} />);

    fireEvent.change(getEmailInput(), {
      target: { value: "not-an-email" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.submit"]() }));

    expect(screen.getByText(m["auth.errors.validEmail"]())).toBeInTheDocument();
    expect(emailCredentialMocks.sendCode).not.toHaveBeenCalled();
  });

  it("does not advance to OTP when Privy rejects the email code request", async () => {
    const onSubmit = vi.fn();
    emailCredentialMocks.sendCode.mockRejectedValue(new Error("email service unavailable"));
    renderWithQueryClient(<EmailHarness onSubmit={onSubmit} />);

    fireEvent.change(getEmailInput(), {
      target: { value: "alice@example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: m["common.submit"]() }));

    await waitFor(() => {
      expect(emailCredentialMocks.sendCode).toHaveBeenCalledWith("alice@example.com");
    });
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("logs in with a six-digit OTP code and authenticates through the configured Privy mode", async () => {
    const onSuccess = vi.fn().mockResolvedValue(undefined);
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={onSuccess} />,
    );

    enterOtp("123456");

    await waitFor(() => {
      expect(emailCredentialMocks.loginWithCode).toHaveBeenCalledWith(
        "alice@example.com",
        "123456",
        "login-or-sign-up",
        {
          embedded: {
            ethereum: {
              createOnLogin: "users-without-wallets",
            },
          },
        },
      );
    });
    expect(emailCredentialMocks.wait).toHaveBeenCalledWith(500);
    expect(onSuccess).toHaveBeenCalledOnce();
  });

  it("does not attempt Privy OTP login before the code is complete", async () => {
    const onSuccess = vi.fn().mockResolvedValue(undefined);
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={onSuccess} />,
    );

    enterOtp("12345");

    await act(async () => {
      await Promise.resolve();
    });

    expect(emailCredentialMocks.loginWithCode).not.toHaveBeenCalled();
    expect(emailCredentialMocks.wait).not.toHaveBeenCalled();
    expect(onSuccess).not.toHaveBeenCalled();
  });

  it("maps Privy OTP failures to the displayed auth error", async () => {
    emailCredentialMocks.loginWithCode.mockRejectedValue(new Error("User does not exist"));
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={vi.fn()} />,
    );

    enterOtp("123456");

    expect(await screen.findByText(m["auth.errors.userNotFound"]())).toBeInTheDocument();
  });

  it("falls back to the generic auth error for unknown OTP provider failures", async () => {
    const onSuccess = vi.fn().mockResolvedValue(undefined);
    emailCredentialMocks.loginWithCode.mockRejectedValue(new Error("provider rate limited"));
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={onSuccess} />,
    );

    enterOtp("123456");

    expect(await screen.findByText(m["auth.errors.authFailed"]())).toBeInTheDocument();
    expect(emailCredentialMocks.wait).not.toHaveBeenCalled();
    expect(onSuccess).not.toHaveBeenCalled();
  });

  it("resends the email code once and enters cooldown", async () => {
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={vi.fn()} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Click to resend" }));

    await waitFor(() => {
      expect(emailCredentialMocks.sendCode).toHaveBeenCalledWith("alice@example.com");
    });
    expect(screen.getByRole("button", { name: "Resend in 00:60" })).toBeDisabled();
  });

  it("blocks repeated resend attempts until the cooldown expires", async () => {
    vi.useFakeTimers();
    renderWithQueryClient(
      <EmailCredential.OTP email="alice@example.com" goBack={vi.fn()} onSuccess={vi.fn()} />,
    );

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Click to resend" }));
      await Promise.resolve();
    });

    expect(emailCredentialMocks.sendCode).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole("button", { name: "Resend in 00:60" }));
    expect(emailCredentialMocks.sendCode).toHaveBeenCalledOnce();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(59_000);
    });

    expect(screen.getByRole("button", { name: "Resend in 00:01" })).toBeDisabled();
    expect(emailCredentialMocks.sendCode).toHaveBeenCalledOnce();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    const resendButton = screen.getByRole("button", { name: "Click to resend" });
    expect(resendButton).toBeEnabled();

    await act(async () => {
      fireEvent.click(resendButton);
      await Promise.resolve();
    });

    expect(emailCredentialMocks.sendCode).toHaveBeenCalledTimes(2);
  });
});
