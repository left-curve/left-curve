import { useState } from "react";
import { View } from "react-native";
import { Signin } from "~/components/Auth/Signin";
import { Signup } from "~/components/Auth/Signup";

export default function AuthScreen() {
  const [activeTab, setActiveTab] = useState("signin");
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8 p-4">
      <View className="h-[40rem] flex flex-col gap-8 w-full items-center justify-center">
        {activeTab === "signin" ? (
          <Signin goTo={() => setActiveTab("signup")} />
        ) : (
          <Signup goTo={() => setActiveTab("signin")} />
        )}
      </View>
    </View>
  );
}
