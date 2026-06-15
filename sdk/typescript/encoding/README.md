# @left-curve/encoding

Encoding and serialization utilities for the [Dango](https://dango.exchange) ecosystem.

## Installation

```bash
npm install @left-curve/encoding
```

## Usage

```typescript
import { encodeHex, decodeHex, encodeBase64, serialize } from "@left-curve/encoding";

const hex = encodeHex(new Uint8Array([0xde, 0xad]));
const bytes = decodeHex("0xdead");

const base64 = encodeBase64(new Uint8Array([1, 2, 3]));

const binary = serialize({ key: "value" });
```

## API

### Hex

- `encodeHex(bytes)` / `decodeHex(hex)` - hex string encoding
- `isHex(value)` - check if string is valid hex
- `hexToBigInt(hex)` - convert hex to bigint

### Base64

- `encodeBase64(bytes)` / `decodeBase64(base64)` - standard base64
- `encodeBase64Url(bytes)` / `decodeBase64Url(base64)` - URL-safe base64
- `base64ToBase64Url(base64)` / `base64UrlToBase64(base64url)` - conversion

### Binary

- `serialize(value)` / `deserialize(binary)` - binary serialization with snake/camel case conversion

### JSON

- `serializeJson(value)` / `deserializeJson(json)` - JSON with superjson support
- `sortedJsonStringify(value)` - deterministic JSON stringification
- `sortedObject(obj)` - recursively sort object keys

### UTF-8

- `encodeUtf8(string)` / `decodeUtf8(bytes)` - UTF-8 encoding

### Endian

- `encodeEndian32(number)` / `decodeEndian32(bytes)` - 32-bit big-endian encoding

### Uint

- `encodeUint(value)` - encode unsigned integer

## License

TBD
