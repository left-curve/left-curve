import { View, Text, Pressable } from "react-native";
import { Outlet, useRouter, useMatches } from "@tanstack/react-router";
import { twMerge } from "@left-curve/foundation";
import { useAccount } from "@left-curve/store";
import { Card } from "../components";
import { Divider } from "../components";

const NAV_ITEMS = [
  { id: "overview", label: "Overview", path: "/account/overview" },
  { id: "portfolio", label: "Portfolio", path: "/account/portfolio" },
  { id: "preferences", label: "Preferences", path: "/account/preferences" },
  { id: "security", label: "Security", path: "/account/security" },
  { id: "session", label: "Session", path: "/account/session" },
  { id: "referral", label: "Referral", path: "/account/referral" },
  { id: "rewards", label: "Rewards", path: "/account/rewards" },
] as const;

function SidebarItem({
  label,
  isActive,
  onPress,
}: {
  label: string;
  isActive: boolean;
  onPress: () => void;
}) {
  return (
    <Pressable
      aria-current={isActive ? "page" : undefined}
      onPress={onPress}
      className={twMerge(
        "flex flex-row items-center",
        "h-9 px-3 rounded-field",
        "transition-[background,color] duration-150 ease-[var(--ease)]",
        isActive
          ? "bg-bg-tint text-fg-primary"
          : "bg-transparent text-fg-secondary hover:bg-bg-sunk hover:text-fg-primary",
      )}
    >
      <Text
        className={twMerge(
          "text-[13px] font-medium",
          isActive ? "text-fg-primary" : "text-fg-secondary",
        )}
      >
        {label}
      </Text>
    </Pressable>
  );
}

function MobileNav({
  activeId,
  onNavigate,
}: {
  activeId: string;
  onNavigate: (path: string) => void;
}) {
  return (
    <View className="flex flex-row gap-1 overflow-x-auto px-4 py-2 md:hidden">
      {NAV_ITEMS.map((item) => (
        <Pressable
          key={item.id}
          onPress={() => onNavigate(item.path)}
          className={twMerge(
            "h-8 px-3 rounded-btn",
            "inline-flex items-center justify-center",
            "text-[12px] font-medium whitespace-nowrap",
            "transition-[background,color] duration-150 ease-[var(--ease)]",
            activeId === item.id
              ? "bg-bg-elev text-fg-primary shadow-sm"
              : "bg-transparent text-fg-tertiary hover:text-fg-secondary",
          )}
        >
          <Text
            className={twMerge(
              "text-[12px] font-medium",
              activeId === item.id ? "text-fg-primary" : "text-fg-tertiary",
            )}
          >
            {item.label}
          </Text>
        </Pressable>
      ))}
    </View>
  );
}

export function AccountLayout() {
  const router = useRouter();
  const matches = useMatches();
  const { username, account } = useAccount();

  const activeId =
    NAV_ITEMS.find((item) => matches.some((m) => m.fullPath === item.path))?.id ?? "overview";

  const handleNavigate = (path: string) => {
    router.navigate({ to: path });
  };

  const displayName = username ?? "Account";
  const truncatedAddress = account?.address
    ? `${account.address.slice(0, 6)}...${account.address.slice(-4)}`
    : "";
  const initials = displayName.slice(0, 2).toUpperCase();

  return (
    <View className="flex-1 max-w-[1640px] mx-auto w-full p-4">
      <MobileNav activeId={activeId} onNavigate={handleNavigate} />

      <View className="flex flex-row gap-4">
        <Card className="hidden md:flex w-[240px] shrink-0 p-4 self-start">
          <View className="flex flex-row items-center gap-3 pb-4">
            <View className="w-10 h-10 rounded-card bg-accent-bg items-center justify-center">
              <Text className="text-accent font-display font-semibold text-[14px]">{initials}</Text>
            </View>
            <View className="flex flex-col gap-0.5 min-w-0">
              <Text className="text-fg-primary font-semibold text-[14px] tracking-tight">
                {displayName}
              </Text>
              <Text className="text-fg-tertiary text-[11px]">{truncatedAddress}</Text>
            </View>
          </View>

          <Divider />

          <View className="flex flex-col gap-0.5 mt-3">
            {NAV_ITEMS.map((item) => (
              <SidebarItem
                key={item.id}
                label={item.label}
                isActive={activeId === item.id}
                onPress={() => handleNavigate(item.path)}
              />
            ))}
          </View>
        </Card>

        <View className="flex-1 min-w-0">
          <Outlet />
        </View>
      </View>
    </View>
  );
}
