import { View, Text, Pressable } from "react-native";
import { Card, Toggle, Divider, Button } from "../components";
import { useNovaTheme } from "../layout/useNovaTheme";
import { useNovaEnabled } from "../useNovaEnabled";
import { getLocale, locales, setLocale } from "@left-curve/foundation/paraglide/runtime.js";
import { useState } from "react";

const LOCALE_LABELS: Record<string, string> = {
  en: "English (US)",
  es: "Espa\u00f1ol",
  fr: "Fran\u00e7ais",
  de: "Deutsch",
  ja: "\u65E5\u672C\u8A9E",
  ko: "\uD55C\uAD6D\uC5B4",
  zh: "\u4E2D\u6587",
};

type SettingRowProps = {
  readonly label: string;
  readonly description: string;
  readonly checked: boolean;
  readonly onChange: (checked: boolean) => void;
};

function SettingRow({ label, description, checked, onChange }: SettingRowProps) {
  return (
    <View className="flex flex-row items-center justify-between py-3 px-1">
      <View className="flex flex-col gap-0.5 flex-1 mr-4">
        <Text className="text-fg-primary text-[13px] font-medium">{label}</Text>
        <Text className="text-fg-tertiary text-[12px]">{description}</Text>
      </View>
      <Toggle checked={checked} onChange={onChange} />
    </View>
  );
}

function SectionTitle({ children }: { children: string }) {
  return (
    <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">{children}</Text>
  );
}

function ThemeOption({
  mode,
  label,
  isActive,
  onPress,
}: {
  mode: "light" | "dark";
  label: string;
  isActive: boolean;
  onPress: () => void;
}) {
  return (
    <View
      className={`flex flex-col items-center gap-2 p-3 rounded-card border cursor-pointer transition-[border-color,background] duration-150 ease-[var(--ease)] ${
        isActive
          ? "border-accent bg-accent-bg"
          : "border-border-default bg-bg-surface hover:border-border-strong"
      }`}
      onStartShouldSetResponder={() => true}
      onResponderRelease={onPress}
    >
      <View
        className={`w-16 h-10 rounded-field ${
          mode === "light" ? "bg-[#fbf6ec]" : "bg-[#181412]"
        } border border-border-subtle items-center justify-center`}
      >
        <View
          className={`w-8 h-2 rounded-full ${mode === "light" ? "bg-[#1f1a16]" : "bg-[#f4ebdc]"}`}
        />
      </View>
      <Text className={`text-[12px] font-medium ${isActive ? "text-accent" : "text-fg-secondary"}`}>
        {label}
      </Text>
    </View>
  );
}

export function Preferences() {
  const { mode, toggle } = useNovaTheme();
  const { toggle: toggleNova } = useNovaEnabled();
  const [notifications, setNotifications] = useState({
    tradeConfirmations: true,
    priceAlerts: true,
    fundingPayments: false,
    depositWithdraw: true,
    marketing: false,
  });

  const toggleNotification = (key: keyof typeof notifications) => {
    setNotifications((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  return (
    <View className="flex flex-col gap-4">
      <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
        Preferences
      </Text>

      <Card className="p-5">
        <SectionTitle>Interface</SectionTitle>
        <View className="flex flex-row items-center justify-between mt-3 p-3 bg-bg-sunk rounded-field">
          <View className="flex flex-col gap-0.5 flex-1 mr-4">
            <Text className="text-fg-primary text-[13px] font-medium">Nova UI</Text>
            <Text className="text-fg-tertiary text-[12px]">
              You are using the new interface. Switch back to the classic version.
            </Text>
          </View>
          <Button variant="secondary" size="sm" onPress={toggleNova}>
            <Text className="text-fg-primary text-[12px]">Switch to Classic</Text>
          </Button>
        </View>
      </Card>

      <Card className="p-5">
        <SectionTitle>Appearance</SectionTitle>
        <View className="flex flex-row gap-3 mt-3">
          <ThemeOption
            mode="light"
            label="Light"
            isActive={mode === "light"}
            onPress={() => mode !== "light" && toggle()}
          />
          <ThemeOption
            mode="dark"
            label="Dark"
            isActive={mode === "dark"}
            onPress={() => mode !== "dark" && toggle()}
          />
        </View>
      </Card>

      <Card className="p-5">
        <SectionTitle>Language</SectionTitle>
        <View className="flex flex-col gap-1 mt-3">
          {locales.map((locale) => {
            const isActive = locale === getLocale();
            return (
              <Pressable
                key={locale}
                onPress={() => setLocale(locale)}
                className={`flex flex-row items-center justify-between p-3 rounded-field transition-[background,border-color] duration-150 ease-[var(--ease)] ${
                  isActive
                    ? "bg-accent-bg border border-accent"
                    : "bg-bg-sunk border border-transparent hover:border-border-strong"
                }`}
              >
                <Text
                  className={`text-[13px] font-medium ${isActive ? "text-accent" : "text-fg-secondary"}`}
                >
                  {LOCALE_LABELS[locale] ?? locale}
                </Text>
                {isActive && <View className="w-2 h-2 rounded-full bg-accent" />}
              </Pressable>
            );
          })}
        </View>
      </Card>

      <Card className="p-5">
        <SectionTitle>Notifications</SectionTitle>
        <View className="flex flex-col mt-2">
          <SettingRow
            label="Trade confirmations"
            description="Get notified when trades are filled"
            checked={notifications.tradeConfirmations}
            onChange={() => toggleNotification("tradeConfirmations")}
          />
          <Divider />
          <SettingRow
            label="Price alerts"
            description="Receive alerts when price targets are hit"
            checked={notifications.priceAlerts}
            onChange={() => toggleNotification("priceAlerts")}
          />
          <Divider />
          <SettingRow
            label="Funding payments"
            description="Notify on perpetual funding rate payments"
            checked={notifications.fundingPayments}
            onChange={() => toggleNotification("fundingPayments")}
          />
          <Divider />
          <SettingRow
            label="Deposit / Withdraw"
            description="Get notified on incoming and outgoing transfers"
            checked={notifications.depositWithdraw}
            onChange={() => toggleNotification("depositWithdraw")}
          />
          <Divider />
          <SettingRow
            label="Marketing"
            description="Product updates, new features, and promotions"
            checked={notifications.marketing}
            onChange={() => toggleNotification("marketing")}
          />
        </View>
      </Card>
    </View>
  );
}
