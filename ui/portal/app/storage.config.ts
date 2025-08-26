import { MMKV, Mode } from "react-native-mmkv";

export const storage = new MMKV({
  //id: `user-${userId}-storage`,
  id: "mmkv.default",
  //encryptionKey: process.env.ENCRYPTION_KEY,
  mode: Mode.MULTI_PROCESS,
  readOnly: false,
});
