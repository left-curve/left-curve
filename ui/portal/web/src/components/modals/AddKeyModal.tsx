import { use } from "react";

import { AddKey } from "./AddKey";

import { m } from "@left-curve/foundation/paraglide/messages.js";

function AddKeyModalContent() {
  const {
    state: { screen },
  } = use(AddKey.Context);

  if (screen === "email-input") {
    return (
      <AddKey.Frame>
        <AddKey.Header
          title={m["settings.keyManagement.email.title"]()}
          description={m["settings.keyManagement.email.description"]()}
        />
        <AddKey.Options>
          <AddKey.EmailInput />
        </AddKey.Options>
      </AddKey.Frame>
    );
  }

  if (screen === "email-otp") {
    return (
      <AddKey.Frame>
        <AddKey.Header
          title={m["settings.keyManagement.email.title"]()}
          description={m["signin.sentVerificationCode"]()}
        />
        <AddKey.Options>
          <AddKey.EmailOtp />
        </AddKey.Options>
      </AddKey.Frame>
    );
  }

  if (screen === "wallets") {
    return (
      <AddKey.Frame>
        <AddKey.Header
          title={m["settings.keyManagement.wallets.title"]()}
          description={m["settings.keyManagement.wallets.description"]()}
        />
        <AddKey.Options>
          <AddKey.WalletsPicker />
        </AddKey.Options>
      </AddKey.Frame>
    );
  }

  if (screen === "public-key-warning") {
    return (
      <AddKey.Frame>
        <AddKey.Header
          title={m["settings.keyManagement.publicKey.warning.title"]()}
          variant="warning"
        />
        <AddKey.Options>
          <AddKey.PublicKeyWarning />
        </AddKey.Options>
      </AddKey.Frame>
    );
  }

  if (screen === "public-key-input") {
    return (
      <AddKey.Frame>
        <AddKey.Header
          title={m["settings.keyManagement.publicKey.input.title"]()}
          description={m["settings.keyManagement.publicKey.input.description"]()}
        />
        <AddKey.Options>
          <AddKey.PublicKeyInput />
        </AddKey.Options>
      </AddKey.Frame>
    );
  }

  return (
    <AddKey.Frame>
      <AddKey.Header
        title={m["settings.keyManagement.management.add.title"]()}
        description={m["settings.keyManagement.management.add.description"]()}
      />
      <AddKey.Options>
        <AddKey.Passkey />
        <AddKey.Email />
        <AddKey.Wallets />
        <AddKey.Advanced />
      </AddKey.Options>
    </AddKey.Frame>
  );
}

export function AddKeyModal() {
  return (
    <AddKey.Provider>
      <AddKeyModalContent />
    </AddKey.Provider>
  );
}
