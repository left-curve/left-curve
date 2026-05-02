import { useCallback, useMemo } from "react";
import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useAccount, useAuthState, useDisconnect } from "@left-curve/store";

import { WelcomeStep } from "./WelcomeStep";
import { PasskeyStep } from "./PasskeyStep";
import { CreateAccountStep } from "./CreateAccountStep";
import { AccountPickerStep } from "./AccountPickerStep";
import { ConnectedStep } from "./ConnectedStep";
import { LoadingStep } from "./LoadingStep";
import { PasskeyErrorStep } from "./PasskeyErrorStep";
import { WalletPickerStep } from "./WalletPickerStep";

import type { AuthScreen } from "@left-curve/store";

const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

type AuthModalProps = {
  readonly isOpen: boolean;
  readonly onClose: () => void;
};

const STEP_META: ReadonlyArray<{ id: AuthScreen | "connected"; label: string }> = [
  { id: "options", label: "Connect" },
  { id: "passkey-choice", label: "Passkey" },
  { id: "account-picker", label: "Account" },
  { id: "create-account", label: "Create" },
  { id: "connected", label: "Done" },
] as const;

const NON_CLOSABLE_STEPS: ReadonlySet<string> = new Set(["create-account"]);

export function AuthModal({ isOpen, onClose }: AuthModalProps) {
  const account = useAccount();
  const { disconnect } = useDisconnect();

  const authState = useAuthState({
    session: true,
    expiration: DEFAULT_SESSION_EXPIRATION,
    onSuccess: () => {
      // Auth succeeded -- modal stays open on connected view
    },
    onError: (e) => {
      console.error("Auth error:", e);
    },
  });

  const {
    screen,
    setScreen,
    email,
    setEmail,
    authenticate,
    passkeyCreate,
    passkeyLogin,
    createAccount,
    selectAccount,
    createNewWithExistingKey,
    users,
    identifier,
    isPending,
    referrer,
    setReferrer,
  } = authState;

  const isConnected = account.isConnected;
  const activeStep = isConnected ? "connected" : screen;

  const currentStepIndex = useMemo(
    () => STEP_META.findIndex((s) => s.id === activeStep),
    [activeStep],
  );

  const canCloseOnScrim = !NON_CLOSABLE_STEPS.has(activeStep) && !isPending;

  const handleScrimPress = useCallback(() => {
    if (canCloseOnScrim) onClose();
  }, [canCloseOnScrim, onClose]);

  const handleClose = useCallback(() => {
    if (!isConnected) {
      setScreen("options");
      setEmail("");
    }
    onClose();
  }, [isConnected, setScreen, setEmail, onClose]);

  const handleDisconnect = useCallback(() => {
    disconnect({});
    setScreen("options");
    setEmail("");
  }, [disconnect, setScreen, setEmail]);

  const renderStep = () => {
    if (account.isConnected && account.username) {
      return (
        <ConnectedStep
          username={account.username}
          onDisconnect={handleDisconnect}
          onClose={handleClose}
        />
      );
    }

    switch (screen) {
      case "options":
        return (
          <WelcomeStep
            email={email}
            onEmailChange={setEmail}
            onContinueEmail={() => setScreen("email")}
            onConnectPasskey={() => authenticate.mutateAsync("passkey")}
            onConnectWallet={() => setScreen("wallets")}
            onConnectPrivy={() => authenticate.mutateAsync("privy")}
            isPending={authenticate.isPending}
          />
        );
      case "email":
        // Privy handles the email/OTP flow through its own UI.
        // Show the welcome step while privy processes.
        return (
          <WelcomeStep
            email={email}
            onEmailChange={setEmail}
            onContinueEmail={() => authenticate.mutateAsync("privy")}
            onConnectPasskey={() => authenticate.mutateAsync("passkey")}
            onConnectWallet={() => setScreen("wallets")}
            onConnectPrivy={() => authenticate.mutateAsync("privy")}
            isPending={authenticate.isPending}
          />
        );
      case "wallets":
        return (
          <WalletPickerStep
            onSelectWallet={(connectorId) => authenticate.mutateAsync(connectorId)}
            onBack={() => setScreen("options")}
            isPending={authenticate.isPending}
          />
        );
      case "passkey-choice":
        return (
          <PasskeyStep
            onCreatePasskey={() => passkeyCreate.mutateAsync()}
            onUseExisting={() => passkeyLogin.mutateAsync()}
            onBack={() => setScreen("options")}
            isCreating={passkeyCreate.isPending}
            isLogging={passkeyLogin.isPending}
          />
        );
      case "passkey-error":
        return (
          <PasskeyErrorStep
            onCreateAccount={() => passkeyCreate.mutateAsync()}
            onBack={() => setScreen("passkey-choice")}
            isPending={passkeyCreate.isPending}
          />
        );
      case "create-account":
        return (
          <CreateAccountStep
            identifier={identifier}
            referrer={referrer}
            onReferrerChange={setReferrer}
            onContinue={() => createAccount.mutateAsync()}
            onBack={() => setScreen("options")}
            isPending={createAccount.isPending}
          />
        );
      case "account-picker":
        return (
          <AccountPickerStep
            users={users}
            onSelectAccount={(userIndex) => selectAccount.mutateAsync(userIndex)}
            onCreateNew={() => createNewWithExistingKey.mutateAsync()}
            onBack={() => setScreen("options")}
            isSelecting={selectAccount.isPending}
            isCreating={createNewWithExistingKey.isPending}
          />
        );
      default:
        return <LoadingStep />;
    }
  };

  if (!isOpen) return null;

  return (
    <View className="fixed inset-0 z-50 items-center justify-center">
      <Pressable
        className="absolute inset-0 bg-black/50"
        onPress={handleScrimPress}
        accessibilityLabel="Close auth modal"
        style={{ animation: "fade 0.2s var(--ease)" } as never}
      />

      <View
        className={twMerge(
          "w-full max-w-[420px] z-10",
          "bg-bg-elev",
          "border border-border-default",
          "rounded-card",
          "shadow-lg",
          "overflow-hidden",
        )}
        role="dialog"
        aria-modal
        aria-label="Sign in to Dango"
        style={{ animation: "pop 0.2s var(--ease)" } as never}
      >
        {/* Top accent line */}
        <View
          className="h-[3px] w-full"
          style={
            {
              backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-soft))",
            } as never
          }
        />

        {/* Step indicator + close button */}
        <View className={twMerge("flex-row items-center justify-between", "px-6 pt-4 pb-2")}>
          <View className="flex-row items-center gap-1.5">
            {STEP_META.map((s, i) => (
              <View
                key={s.id}
                className={twMerge(
                  "h-1 rounded-full transition-all duration-200 ease-[var(--ease)]",
                  i <= currentStepIndex ? "w-5 bg-accent" : "w-1.5 bg-border-default",
                )}
              />
            ))}
          </View>

          <Pressable
            onPress={handleClose}
            className={twMerge(
              "w-7 h-7 items-center justify-center",
              "rounded-btn",
              "hover:bg-bg-tint",
              "transition-colors duration-150 ease-[var(--ease)]",
            )}
            accessibilityLabel="Close"
          >
            <Text className="text-fg-tertiary text-[14px]">{"\u2715"}</Text>
          </Pressable>
        </View>

        {renderStep()}
      </View>
    </View>
  );
}
