const messages: Record<string, string> = {
  "auth.email": "Email",
  "auth.passkey": "Passkey",
  "auth.wallets": "Wallets",
  "common.back": "Back",
  "common.cancel": "Cancel",
  "common.continue": "Continue",
  "settings.keyManagement.advanced": "Advanced",
  "settings.keyManagement.management.add.error.alreadyExists": "Key already exists.",
  "settings.keyManagement.management.add.error.title": "Couldn't add key",
  "settings.keyManagement.management.add.success.description": "Your key has been added.",
  "settings.keyManagement.management.add.success.title": "Key added",
  "settings.keyManagement.publicKey.input.error":
    "This doesn't look like a valid secp256k1 key. Please check and try again.",
  "settings.keyManagement.publicKey.input.label": "Public Key",
  "settings.keyManagement.publicKey.input.placeholder": "Paste your secp256k1 public key",
  "settings.keyManagement.publicKey.input.submit": "Add key",
  "settings.keyManagement.publicKey.input.valid": "Valid secp256k1 public key",
  "settings.keyManagement.publicKey.option": "Secp256k1 Public Key",
  "settings.keyManagement.publicKey.summary.confirm": "Confirm",
  "settings.keyManagement.publicKey.summary.key": "Public key",
  "settings.keyManagement.publicKey.summary.type": "Type",
  "settings.keyManagement.publicKey.summary.typeValue": "Secp256k1 Public Key",
  "settings.keyManagement.publicKey.warning.confirmations.authority":
    "I understand this key can authorize account actions.",
  "settings.keyManagement.publicKey.warning.confirmations.generated":
    "I generated this key securely.",
  "settings.keyManagement.publicKey.warning.confirmations.privateKey":
    "I control the matching private key.",
  "settings.keyManagement.publicKey.warning.scam":
    "Only add keys you generated and control yourself.",
};

export const m = new Proxy({} as Record<string, () => string>, {
  get: (_target, key) => {
    if (typeof key !== "string") return undefined;
    return () => messages[key] ?? key;
  },
});
