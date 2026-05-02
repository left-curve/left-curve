import { useCallback } from "react";
import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useRouterState, useNavigate } from "@tanstack/react-router";
import { useNovaTheme } from "./useNovaTheme";
import { SearchPalette } from "./SearchPalette";
import { AccountMenu } from "./AccountMenu";
import { useAuth } from "../auth";

const NAV_ITEMS = [
  { label: "Trade", path: "/trade" },
  { label: "Earn", path: "/earn" },
  { label: "Move", path: "/move" },
  { label: "Account", path: "/account" },
  { label: "Explorer", path: "/explorer" },
] as const;

export function Header() {
  const { mode, toggle } = useNovaTheme();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const { showAuth, account } = useAuth();

  const isActive = useCallback(
    (path: string) => pathname === path || pathname.startsWith(`${path}/`),
    [pathname],
  );

  return (
    <View
      className={twMerge(
        "flex-row items-center justify-between",
        "h-12 px-2 pl-6",
        "bg-bg-surface",
        "border-b border-border-subtle",
      )}
    >
      <Pressable onPress={() => navigate({ to: "/trade" })} className="justify-center">
        <img
          src={mode === "dark" ? "/images/dango-dark.svg" : "/images/dango.svg"}
          alt="Dango"
          className="h-7 w-auto select-none drag-none"
        />
      </Pressable>

      <View className="flex-row items-center gap-1">
        {NAV_ITEMS.map((item) => (
          <Pressable
            key={item.path}
            onPress={() => navigate({ to: item.path })}
            className={twMerge(
              "px-3 py-1.5 rounded-btn",
              "transition-colors duration-150 ease-[var(--ease)]",
              isActive(item.path) ? "bg-bg-tint" : "hover:bg-bg-sunk",
            )}
          >
            <Text
              className={twMerge(
                "font-text text-[12px] font-medium",
                isActive(item.path) ? "text-fg-primary" : "text-fg-tertiary",
              )}
            >
              {item.label}
            </Text>
          </Pressable>
        ))}
      </View>

      <View className="flex-row items-center gap-2">
        <SearchPalette.Compact className="hidden md:inline-flex" />

        <Pressable
          onPress={toggle}
          className={twMerge(
            "w-8 h-8 items-center justify-center",
            "rounded-btn",
            "hover:bg-bg-tint",
            "transition-colors duration-150 ease-[var(--ease)]",
          )}
          accessibilityLabel={`Switch to ${mode === "light" ? "dark" : "light"} mode`}
        >
          <Text className="text-fg-secondary text-[14px]">
            {mode === "light" ? "\u263D" : "\u2600"}
          </Text>
        </Pressable>

        {account.isConnected ? (
          <AccountMenu />
        ) : (
          <Pressable
            onPress={showAuth}
            className={twMerge(
              "h-8 px-3 flex-row items-center justify-center gap-2",
              "rounded-chip",
              "transition-[background,border-color] duration-150 ease-[var(--ease)]",
              "bg-bg-tint hover:bg-bg-sunk",
            )}
            accessibilityLabel="Sign in"
          >
            <Text className="font-text text-[12px] font-medium text-fg-secondary" numberOfLines={1}>
              Sign In
            </Text>
          </Pressable>
        )}
      </View>
    </View>
  );
}
