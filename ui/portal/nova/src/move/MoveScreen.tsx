import { useState, useCallback } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { SendTab } from "./SendTab";
import { DepositTab } from "./DepositTab";
import { SpotPerpsTab } from "./SpotPerpsTab";

type MoveTab = "send" | "deposit" | "perps";

type MoveTabConfig = {
  readonly value: MoveTab;
  readonly label: string;
  readonly icon: React.ReactNode;
  readonly hint: string;
};

function ArrowOutIcon({
  size = 16,
  className,
}: {
  readonly size?: number;
  readonly className?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M5 15L15 5M9 5h6v6" />
    </svg>
  );
}

function ArrowInIcon({
  size = 16,
  className,
}: {
  readonly size?: number;
  readonly className?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M15 5L5 15M11 15H5V9" />
    </svg>
  );
}

function SwapIcon({
  size = 16,
  className,
}: {
  readonly size?: number;
  readonly className?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M5 7h11M16 7l-3-3M16 7l-3 3M15 13H4M4 13l3-3M4 13l3 3" />
    </svg>
  );
}

function InfoIcon({
  size = 14,
  className,
}: {
  readonly size?: number;
  readonly className?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <circle cx="10" cy="10" r="7" />
      <path d="M10 9v5M10 6.5h.01" />
    </svg>
  );
}

const MOVE_TABS: readonly MoveTabConfig[] = [
  { value: "send", label: "Send", icon: <ArrowOutIcon size={16} />, hint: "Spot \u2192 out" },
  { value: "deposit", label: "Deposit", icon: <ArrowInIcon size={16} />, hint: "Show address" },
  {
    value: "perps",
    label: "Spot \u21D4 Perps",
    icon: <SwapIcon size={16} />,
    hint: "USDC \u21D4 USD",
  },
];

function MainTabButton({
  tab,
  isActive,
  onPress,
}: {
  readonly tab: MoveTabConfig;
  readonly isActive: boolean;
  readonly onPress: () => void;
}) {
  return (
    <Pressable
      role="tab"
      aria-selected={isActive}
      onPress={onPress}
      className={twMerge(
        "flex-1 flex flex-col items-center gap-1.5 py-3.5 px-3",
        "transition-[background,color,border-color] duration-150 ease-[var(--ease)]",
        isActive
          ? "bg-bg-base border-b-2 border-b-fg-primary"
          : "bg-bg-sunk border-b-2 border-b-transparent",
      )}
    >
      <Text className={twMerge(isActive ? "text-fg-primary" : "text-fg-tertiary")}>{tab.icon}</Text>
      <Text
        className={twMerge(
          "text-sm font-medium font-display",
          isActive ? "text-fg-primary" : "text-fg-tertiary",
        )}
      >
        {tab.label}
      </Text>
      <Text className="text-[10px] text-fg-tertiary">{tab.hint}</Text>
    </Pressable>
  );
}

export function MoveScreen() {
  const [activeTab, setActiveTab] = useState<MoveTab>("send");

  const handleTabChange = useCallback((val: MoveTab) => {
    setActiveTab(val);
  }, []);

  return (
    <View className="flex-1 items-center py-8 px-4">
      <View className="w-full max-w-[480px] flex flex-col gap-6">
        <View className="flex flex-col gap-2">
          <Text className="text-fg-primary font-display text-[28px] font-semibold tracking-tight">
            Move money
          </Text>
          <Text className="text-fg-secondary text-sm leading-relaxed">
            Send to anyone, receive into your Spot, or shuffle USDC between Spot and Perps.
          </Text>
        </View>

        <View className="bg-bg-surface border border-border-subtle rounded-card overflow-hidden">
          <View className="flex flex-row border-b border-border-subtle">
            {MOVE_TABS.map((tab) => (
              <MainTabButton
                key={tab.value}
                tab={tab}
                isActive={activeTab === tab.value}
                onPress={() => handleTabChange(tab.value)}
              />
            ))}
          </View>

          <View className="p-5">
            {activeTab === "send" && <SendTab />}
            {activeTab === "deposit" && <DepositTab />}
            {activeTab === "perps" && <SpotPerpsTab />}
          </View>
        </View>

        <View className="flex flex-row gap-2.5 px-3.5 py-3 bg-bg-tint border border-border-subtle rounded-card">
          <Text className="text-fg-tertiary shrink-0">
            <InfoIcon size={14} />
          </Text>
          <Text className="text-fg-secondary text-sm leading-relaxed flex-1">
            <Text className="text-fg-primary font-medium">Send</Text> moves Spot to another address
            or chain. <Text className="text-fg-primary font-medium">Spot {"\u21D4"} Perps</Text>{" "}
            deposits or withdraws USDC inside one of your accounts (it becomes USD on the Perps
            side).
          </Text>
        </View>
      </View>
    </View>
  );
}
