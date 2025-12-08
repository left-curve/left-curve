import "@formatjs/intl-getcanonicallocales/polyfill";
import "@formatjs/intl-locale/polyfill";

import "@formatjs/intl-pluralrules/polyfill";
import "@formatjs/intl-pluralrules/locale-data/en";

import "@formatjs/intl-numberformat/polyfill";
import "@formatjs/intl-numberformat/locale-data/en";

import QuickCrypto from "react-native-quick-crypto";
import { TextDecoder, TextEncoder } from "text-encoding";

global.crypto = {
  ...QuickCrypto.webcrypto,
  randomUUID: () => QuickCrypto.randomUUID() as `${string}-${string}-${string}-${string}-${string}`,
  getRandomValues: <T extends ArrayBufferView>(array: T) =>
    QuickCrypto.getRandomValues(array as unknown as Uint8Array),
} as unknown as Crypto;

global.TextEncoder = TextEncoder;
global.TextDecoder = TextDecoder;

import { createMMKVStorage } from "~/storage";
global.localStorage = createMMKVStorage() as Storage;

global.BroadcastChannel = class {
  constructor(public name: string) {}
  addEventListener() {}
  removeEventListener() {}
  postMessage() {}
  close() {}
} as unknown as typeof BroadcastChannel;
