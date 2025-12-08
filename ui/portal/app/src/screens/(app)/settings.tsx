import { useRouter } from "expo-router";
import { View } from "react-native";
import { Button, GlobalText, IconChevronDown } from "~/components/foundation";
import { DisplaySection } from "~/components/Settings/DisplaySection";

export default function Settings() {
  const { back } = useRouter();

  return (
    <View className="flex-1 flex bg-surface-primary-rice w-full flex-col gap-8 p-4">
      <View className="flex flex-row gap-2 items-center lg:hidden self-start">
        <Button
          variant="link"
          size="icon"
          classNames={{ icons: "text-ink-tertiary-500 rotate-90" }}
          onPress={back}
          rightIcon={<IconChevronDown className="text-ink-tertiary-500" />}
        />

        <GlobalText className="h3-bold text-ink-primary-900">Settings</GlobalText>
      </View>
      <DisplaySection>
        <DisplaySection.Language />
        <DisplaySection.FormatNumber />
        <DisplaySection.DateFormat />
        <DisplaySection.TimeFormat />
        <DisplaySection.TimeZone />
        <DisplaySection.Theme />
      </DisplaySection>
    </View>
  );
}
