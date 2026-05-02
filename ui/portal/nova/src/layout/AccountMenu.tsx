import { useCallback, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useNavigate } from "@tanstack/react-router";
import {
  useAccount,
  useBalances,
  useDisconnect,
  usePrices,
  useSessionKey,
} from "@left-curve/store";
import { Dropdown } from "../components/Dropdown";
import { Divider } from "../components/Divider";
import { Badge } from "../components/Badge";

import type { Account } from "@left-curve/dango/types";

function truncateAddress(address: string, start = 8, end = 6): string {
  if (address.length <= start + end + 3) return address;
  return `${address.slice(0, start)}...${address.slice(-end)}`;
}

export function AccountMenu() {
  const navigate = useNavigate();
  const { username, account, accounts, changeAccount, connector } = useAccount();
  const { disconnect } = useDisconnect();
  const { deleteSessionKey } = useSessionKey();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  const close = useCallback(() => setOpen(false), []);

  const handleCopyAddress = useCallback(() => {
    if (!account?.address) return;
    navigator.clipboard.writeText(account.address).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [account?.address]);

  const handleSwitchAccount = useCallback(
    (address: string) => {
      changeAccount?.(address);
      close();
    },
    [changeAccount, close],
  );

  const handleDisconnect = useCallback(() => {
    close();
    connector?.disconnect();
    deleteSessionKey();
    disconnect({});
  }, [close, connector, deleteSessionKey, disconnect]);

  const handleNavigate = useCallback(
    (path: string) => {
      navigate({ to: path });
      close();
    },
    [navigate, close],
  );

  if (!account || !username) return null;

  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: { currency: "usd" },
  });
  const otherAccounts = (accounts ?? []).filter((a: Account) => a.address !== account.address);

  return (
    <Dropdown
      open={open}
      onOpenChange={setOpen}
      align="right"
      className="w-[320px] p-2 gap-0.5"
      trigger={
        <Pressable
          onPress={() => setOpen((prev) => !prev)}
          className={twMerge(
            "h-8 px-3 flex-row items-center justify-center gap-2",
            "rounded-chip",
            "transition-[background,border-color] duration-150 ease-[var(--ease)]",
            open
              ? "bg-bg-tint border border-border-strong"
              : "bg-accent-bg border border-transparent hover:border-accent",
          )}
          accessibilityLabel="Account menu"
        >
          <View className="w-2 h-2 rounded-full bg-up" />
          <Text className="font-text text-[12px] font-medium text-accent" numberOfLines={1}>
            {username}
          </Text>
          <Text className="text-fg-tertiary text-[10px]">{"\u25BE"}</Text>
        </Pressable>
      }
    >
      {/* Current account header */}
      <View className="flex-row items-center gap-3 p-3 pb-3.5 border-b border-border-subtle mb-1.5">
        <View
          className="w-10 h-10 rounded-full bg-accent"
          style={{ boxShadow: "0 6px 16px -8px var(--color-accent)" }}
        />
        <View className="flex-1 min-w-0 gap-0.5">
          <View className="flex-row items-center gap-1.5">
            <Text className="font-text text-[14px] font-semibold text-fg-primary" numberOfLines={1}>
              {username}
            </Text>
            <Badge variant="accent">Active</Badge>
          </View>
          <Text className="font-text text-[11px] text-fg-tertiary tabular-nums" numberOfLines={1}>
            {truncateAddress(account.address)}
          </Text>
        </View>
        <Pressable
          onPress={handleCopyAddress}
          className={twMerge(
            "w-7 h-7 rounded-lg items-center justify-center",
            "bg-bg-tint border border-border-subtle",
            "hover:bg-bg-sunk",
            "transition-colors duration-150 ease-[var(--ease)]",
          )}
          accessibilityLabel="Copy address"
        >
          <Text className="text-fg-secondary text-[11px]">{copied ? "\u2713" : "\u2398"}</Text>
        </Pressable>
      </View>

      {/* Balance summary */}
      <View className="px-3 py-2">
        <Text className="font-text text-[10px] font-medium tracking-wide uppercase text-fg-tertiary">
          Total balance
        </Text>
        <Text className="font-text text-[18px] font-semibold text-fg-primary tabular-nums tracking-tight mt-0.5">
          {totalBalance}
        </Text>
      </View>

      <Divider className="mx-2 my-1.5" />

      {/* Account list */}
      {otherAccounts.length > 0 && (
        <>
          <View className="px-3 py-1.5">
            <Text className="font-text text-[10px] font-medium tracking-wide uppercase text-fg-tertiary">
              Switch account
            </Text>
          </View>

          {otherAccounts.map((a: Account) => (
            <Pressable
              key={a.address}
              onPress={() => handleSwitchAccount(a.address)}
              className={twMerge(
                "flex-row items-center gap-2.5 px-2.5 py-2",
                "rounded-[10px]",
                "hover:bg-bg-tint",
                "transition-colors duration-100 ease-[var(--ease)]",
              )}
              accessibilityRole="menuitem"
            >
              <View
                className="w-7 h-7 rounded-full shrink-0"
                style={{
                  backgroundColor: `oklch(${65 - a.index * 6}% 0.14 ${60 + a.index * 30})`,
                }}
              />
              <View className="flex-1 min-w-0 gap-px">
                <View className="flex-row items-center gap-1.5">
                  <Text
                    className="font-text text-[13px] font-medium text-fg-primary"
                    numberOfLines={1}
                  >
                    {username}#{a.index}
                  </Text>
                  <Text className="font-text text-[9px] font-medium tracking-wide uppercase text-fg-tertiary">
                    {a.index === 0 ? "Spot" : "Perps"}
                  </Text>
                </View>
                <Text
                  className="font-text text-[10px] text-fg-tertiary tabular-nums"
                  numberOfLines={1}
                >
                  {truncateAddress(a.address)}
                </Text>
              </View>
              <Text className="text-fg-tertiary text-[12px]">{"\u2197"}</Text>
            </Pressable>
          ))}

          <Divider className="mx-2 my-1.5" />
        </>
      )}

      {/* Actions */}
      <MenuAction
        icon="+"
        label="Create new account"
        onPress={() => handleNavigate("/account/create")}
      />
      <MenuAction
        icon={"\u2699"}
        label="Account settings"
        onPress={() => handleNavigate("/account/settings")}
      />
      <MenuAction icon={"\u26A1"} label="Portfolio" onPress={() => handleNavigate("/account")} />

      <Divider className="mx-2 my-1.5" />

      {/* Disconnect */}
      <Pressable
        onPress={handleDisconnect}
        className={twMerge(
          "flex-row items-center gap-2.5 px-3 py-2.5",
          "rounded-[10px]",
          "hover:bg-down/[0.08]",
          "transition-colors duration-100 ease-[var(--ease)]",
        )}
        accessibilityRole="menuitem"
      >
        <View className="w-7 h-7 rounded-lg bg-down/[0.12] items-center justify-center">
          <Text className="text-down text-[12px]">{"\u21A9"}</Text>
        </View>
        <Text className="font-text text-[13px] font-medium text-down">Disconnect</Text>
      </Pressable>
    </Dropdown>
  );
}

type MenuActionProps = {
  readonly icon: string;
  readonly label: string;
  readonly onPress: () => void;
};

function MenuAction({ icon, label, onPress }: MenuActionProps) {
  return (
    <Pressable
      onPress={onPress}
      className={twMerge(
        "flex-row items-center gap-2.5 px-3 py-2.5",
        "rounded-[10px]",
        "hover:bg-bg-tint",
        "transition-colors duration-100 ease-[var(--ease)]",
      )}
      accessibilityRole="menuitem"
    >
      <View
        className={twMerge(
          "w-7 h-7 rounded-lg items-center justify-center",
          "bg-bg-tint border border-border-subtle",
        )}
      >
        <Text className="text-fg-secondary text-[12px]">{icon}</Text>
      </View>
      <Text className="font-text text-[13px] font-medium text-fg-primary">{label}</Text>
    </Pressable>
  );
}
