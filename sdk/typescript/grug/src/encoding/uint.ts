export function encodeUint(number: string, byteLength: 32 | 16): Uint8Array {
  let bigInt = BigInt(number);
  const byteArray = new Uint8Array(byteLength);

  for (let i = 0; i < byteLength; i++) {
    byteArray[byteLength - 1 - i] = Number(bigInt & BigInt(0xff));
    bigInt >>= BigInt(8);
  }

  return byteArray;
}
