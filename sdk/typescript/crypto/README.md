# @left-curve/crypto

Cryptographic primitives for the [Dango](https://dango.exchange) ecosystem.

## Installation

```bash
npm install @left-curve/crypto
```

## Usage

```typescript
import { sha256, Secp256k1, Ed25519 } from "@left-curve/crypto";

const hash = sha256(new Uint8Array([1, 2, 3]));

const keypair = Secp256k1.fromMnemonic("your mnemonic ...");
const pubkey = keypair.publicKey();
```

## API

### Hash Functions

- `sha256(data)` / `Sha256` - SHA-256
- `sha512(data)` / `Sha512` - SHA-512
- `keccak256(data)` / `Keccak256` - Keccak-256
- `ripemd160(data)` / `Ripemd160` - RIPEMD-160

### Key Pairs

- `Secp256k1` - secp256k1 elliptic curve key pair
  - `Secp256k1.fromMnemonic(mnemonic)` - derive from BIP-39 mnemonic
  - `Secp256k1.fromPrivateKey(key)` - from raw private key
  - `.publicKey()`, `.sign(data)`, `.verify(data, signature)`
- `Ed25519` - Ed25519 key pair
  - `.publicKey()`, `.sign(data)`, `.verify(data, signature)`

### Utilities

- `secp256k1RecoverPubKey(hash, signature, recoveryId)` - recover public key
- `secp256k1CompressPubKey(pubkey)` - compress public key
- `secp256k1VerifySignature(pubkey, hash, signature)` - verify signature
- `ed25519VerifySignature(pubkey, hash, signature)` - verify signature
- `ethHashMessage(message)` - EIP-191 message hashing

### WebAuthn

- `createWebAuthnCredential(options)` - create passkey credential
- `signWithWebAuthn(credential, challenge)` - sign with passkey
- `verifyWebAuthn(credential, signature)` - verify passkey signature

## Dependencies

Built on audited libraries:

- [`@noble/curves`](https://github.com/paulmillr/noble-curves) - elliptic curves
- [`@noble/hashes`](https://github.com/paulmillr/noble-hashes) - hash functions
- [`@scure/bip32`](https://github.com/paulmillr/scure-bip32) - HD key derivation
- [`@scure/bip39`](https://github.com/paulmillr/scure-bip39) - mnemonic generation

## License

TBD
