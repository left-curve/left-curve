import { useRouter } from "expo-router";
import { View } from "react-native";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Button, GlobalText } from "~/components/foundation";

type AuthenticateSheetProps = {
  action?: "signin" | "signup";
  closeSheet: () => void;
};

export const AuthenticateSheet: React.FC<AuthenticateSheetProps> = ({ closeSheet }) => {
  const router = useRouter();

  return (
    <View className="flex flex-col gap-4">
      <GlobalText className="diatype-sm-regular text-ink-tertiary-500 text-center">
        {m["signin.connectWalletToContinue"]()}
      </GlobalText>
      <Button
        onPress={() => {
          closeSheet();
          router.push("/auth");
        }}
      >
        {m["common.signin"]()}
      </Button>
    </View>
  );
};
