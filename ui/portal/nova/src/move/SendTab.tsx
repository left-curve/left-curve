import { useState, useCallback, useMemo } from "react";
import { View, Text, Pressable, TextInput } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, formatUnits, parseUnits } from "@left-curve/dango/utils";
import {
  useAccount,
  useBalances,
  useConfig,
  usePrices,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import type { Address } from "@left-curve/dango/types";
import { Button } from "../components";

type DestinationType = "address" | "account" | "bridge";

type TokenEntry = {
  readonly denom: string;
  readonly symbol: string;
  readonly name: string;
  readonly decimals: number;
  readonly logoURI?: string;
  readonly humanBalance: string;
};

type DestTabConfig = {
  readonly value: DestinationType;
  readonly label: string;
  readonly icon: string;
  readonly hint: string;
};

const DEST_TABS: readonly DestTabConfig[] = [
  { value: "address", label: "Address", icon: "\u{1F5A5}", hint: "Onchain" },
  { value: "account", label: "My Spot", icon: "\u{1F464}", hint: "Other account" },
  { value: "bridge", label: "Bridge", icon: "\u2197", hint: "Other chain" },
];

const CHAINS = ["Ethereum", "Arbitrum", "Base", "Optimism"] as const;

const NETWORK_INFO: Record<DestinationType, string> = {
  address: "Onchain \u00B7 Dango network \u00B7 ~$0.18 fee",
  account: "Internal \u00B7 instant \u00B7 no fee",
  bridge: "Bridge \u00B7 ~30s \u00B7 network fee applies",
};

function formatDisplay(value: string, decimals = 2): string {
  return Number(Number(value).toFixed(decimals)).toLocaleString("en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

function AccountAvatar({ letter }: { readonly letter: string }) {
  return (
    <View className="w-7 h-7 rounded-field bg-up-bg items-center justify-center">
      <Text className="text-up font-mono font-semibold text-xs">{letter}</Text>
    </View>
  );
}

function TokenIcon({ symbol, logoURI }: { readonly symbol: string; readonly logoURI?: string }) {
  if (logoURI) {
    return (
      <View className="w-5 h-5 rounded-full overflow-hidden">
        <img src={logoURI} alt={symbol} className="w-full h-full object-cover" />
      </View>
    );
  }
  return (
    <View className="w-5 h-5 rounded-full bg-fg-primary items-center justify-center">
      <Text className="text-bg-surface text-[9px] font-semibold">{symbol.slice(0, 1)}</Text>
    </View>
  );
}

function DestTabButton({
  tab,
  isActive,
  onPress,
}: {
  readonly tab: DestTabConfig;
  readonly isActive: boolean;
  readonly onPress: () => void;
}) {
  return (
    <Pressable
      role="tab"
      aria-selected={isActive}
      onPress={onPress}
      className={twMerge(
        "flex-1 flex flex-col items-center gap-1 py-2.5 px-2 rounded-field",
        "transition-[background,color,box-shadow] duration-150 ease-[var(--ease)]",
        isActive ? "bg-bg-surface shadow-sm" : "bg-transparent hover:bg-bg-tint",
      )}
    >
      <Text className={twMerge("text-md", isActive ? "text-fg-primary" : "text-fg-secondary")}>
        {tab.icon}
      </Text>
      <Text
        className={twMerge(
          "text-sm font-medium font-display",
          isActive ? "text-fg-primary" : "text-fg-secondary",
        )}
      >
        {tab.label}
      </Text>
      <Text className="text-2xs text-fg-tertiary">{tab.hint}</Text>
    </Pressable>
  );
}

function AddressDestination({
  value,
  onChangeText,
}: {
  readonly value: string;
  readonly onChangeText: (text: string) => void;
}) {
  const handleCopy = useCallback(() => {
    navigator.clipboard
      .readText()
      .then((text) => onChangeText(text))
      .catch(() => {});
  }, [onChangeText]);

  return (
    <View
      className={twMerge(
        "flex flex-row items-center gap-2.5",
        "px-3 h-10",
        "bg-bg-surface border border-border-default rounded-field",
        "focus-within:border-fg-primary focus-within:bg-bg-elev",
        "transition-[border-color,background] duration-150 ease-[var(--ease)]",
      )}
    >
      <View className="w-7 h-7 rounded-field bg-bg-tint items-center justify-center shrink-0">
        <Text className="text-fg-secondary text-xs">{"\u{1F5A5}"}</Text>
      </View>
      <TextInput
        value={value}
        onChangeText={onChangeText}
        placeholder="Wallet address or username"
        placeholderTextColor="var(--fg-quaternary)"
        className="flex-1 min-w-0 bg-transparent border-0 outline-none font-mono text-sm text-fg-primary"
      />
      <Pressable onPress={handleCopy} className="p-1 hover:bg-bg-tint rounded-field">
        <Text className="text-fg-tertiary text-xs">{"\u{1F4CB}"}</Text>
      </Pressable>
      <Pressable className="p-1 hover:bg-bg-tint rounded-field">
        <Text className="text-fg-tertiary text-xs">{"\u{2B1A}"}</Text>
      </Pressable>
    </View>
  );
}

function AccountDestination({
  value,
  onChange,
  accounts,
  currentAddress,
}: {
  readonly value: string;
  readonly onChange: (address: string) => void;
  readonly accounts: readonly { readonly address: string; readonly index: number }[];
  readonly currentAddress: string;
}) {
  const otherAccounts = accounts.filter((a) => a.address !== currentAddress);

  return (
    <View className="flex flex-col gap-1.5">
      {otherAccounts.map((acct) => {
        const letter = String(acct.index);
        return (
          <Pressable
            key={acct.address}
            onPress={() => onChange(acct.address)}
            className={twMerge(
              "flex flex-row items-center gap-3 px-3 py-2.5 rounded-field",
              "border transition-[border-color] duration-150 ease-[var(--ease)]",
              value === acct.address
                ? "border-fg-primary bg-bg-surface"
                : "border-border-default bg-bg-surface hover:border-border-strong",
            )}
          >
            <AccountAvatar letter={letter} />
            <View className="flex-1 flex flex-col gap-0.5">
              <Text className="text-fg-primary text-sm font-medium font-display">
                Account {acct.index} {"\u00B7"} Spot
              </Text>
              <Text className="text-fg-tertiary text-xs font-mono">
                {acct.address.slice(0, 10)}...{acct.address.slice(-6)}
              </Text>
            </View>
            {value === acct.address && <Text className="text-fg-primary text-sm">{"\u2713"}</Text>}
          </Pressable>
        );
      })}
      {otherAccounts.length === 0 && (
        <Text className="text-xs text-fg-tertiary px-0.5">No other accounts available</Text>
      )}
    </View>
  );
}

function BridgeDestination({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (chain: string) => void;
}) {
  return (
    <View style={{ display: "grid" as never, gridTemplateColumns: "1fr 1fr", gap: 6 }}>
      {CHAINS.map((chain) => (
        <Pressable
          key={chain}
          onPress={() => onChange(chain)}
          className={twMerge(
            "flex flex-row items-center gap-2.5 px-3 py-2.5 rounded-field",
            "border transition-[border-color] duration-150 ease-[var(--ease)]",
            value === chain
              ? "border-fg-primary bg-bg-surface"
              : "border-border-default bg-bg-surface hover:border-border-strong",
          )}
        >
          <View className="w-[22px] h-[22px] rounded-full bg-bg-tint items-center justify-center">
            <Text className="text-fg-secondary text-2xs font-semibold font-mono">
              {chain.slice(0, 2)}
            </Text>
          </View>
          <Text className="flex-1 text-fg-primary text-sm font-medium font-display">{chain}</Text>
          {value === chain && <Text className="text-fg-primary text-sm">{"\u2713"}</Text>}
        </Pressable>
      ))}
    </View>
  );
}

function TokenPicker({
  value,
  onChange,
  tokens,
}: {
  readonly value: TokenEntry;
  readonly onChange: (token: TokenEntry) => void;
  readonly tokens: readonly TokenEntry[];
}) {
  const [open, setOpen] = useState(false);

  const handleSelect = useCallback(
    (token: TokenEntry) => {
      onChange(token);
      setOpen(false);
    },
    [onChange],
  );

  return (
    <View className="relative">
      <Pressable
        onPress={() => setOpen((prev) => !prev)}
        className={twMerge(
          "flex flex-row items-center gap-1.5",
          "px-2 py-1.5",
          "bg-bg-tint rounded-field",
          "transition-[background] duration-150 ease-[var(--ease)]",
          "hover:bg-bg-sunk",
        )}
      >
        <TokenIcon symbol={value.symbol} logoURI={value.logoURI} />
        <Text className="text-fg-primary text-sm font-medium font-display">{value.symbol}</Text>
        <Text className="text-fg-tertiary text-xs">{"\u25BE"}</Text>
      </Pressable>
      {open && (
        <View
          className={twMerge(
            "absolute top-full right-0 mt-1 min-w-[180px] z-10",
            "bg-bg-elev border border-border-default rounded-card shadow-md",
            "p-1",
          )}
        >
          {tokens.map((token) => (
            <Pressable
              key={token.denom}
              onPress={() => handleSelect(token)}
              className={twMerge(
                "flex flex-row items-center gap-2.5 px-2.5 py-2 rounded-field",
                "transition-[background] duration-150 ease-[var(--ease)]",
                token.denom === value.denom ? "bg-bg-tint" : "hover:bg-bg-sunk",
              )}
            >
              <TokenIcon symbol={token.symbol} logoURI={token.logoURI} />
              <View className="flex-1 flex flex-col">
                <Text className="text-fg-primary text-sm font-medium">{token.symbol}</Text>
                <Text className="text-fg-tertiary text-2xs">{token.name}</Text>
              </View>
              <Text className="text-fg-tertiary text-2xs font-mono">
                {formatDisplay(token.humanBalance, 2)}
              </Text>
            </Pressable>
          ))}
        </View>
      )}
    </View>
  );
}

export function SendTab() {
  const { account, accounts, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: balances = {} } = useBalances({ address: account?.address as Address });
  const { getPrice } = usePrices();
  const { data: signingClient } = useSigningClient();

  const tokenEntries: readonly TokenEntry[] = useMemo(() => {
    const denoms = Object.keys(balances).length > 0 ? Object.keys(balances) : ["bridge/usdc"];
    return denoms
      .map((denom) => {
        const coin = coins.byDenom[denom];
        if (!coin) return null;
        const rawBalance = balances[denom] || "0";
        const humanBalance = formatUnits(rawBalance, coin.decimals);
        return {
          denom: coin.denom,
          symbol: coin.symbol,
          name: coin.name,
          decimals: coin.decimals,
          logoURI: coin.logoURI,
          humanBalance,
        };
      })
      .filter((t): t is TokenEntry => t !== null);
  }, [balances, coins.byDenom]);

  const defaultToken: TokenEntry = tokenEntries[0] ?? {
    denom: "bridge/usdc",
    symbol: "USDC",
    name: "USD Coin",
    decimals: 6,
    logoURI: undefined,
    humanBalance: "0",
  };

  const [destType, setDestType] = useState<DestinationType>("address");
  const [selectedDenom, setSelectedDenom] = useState(defaultToken.denom);
  const [amount, setAmount] = useState("");
  const [address, setAddress] = useState("");
  const [selectedAccount, setSelectedAccount] = useState("");
  const [selectedChain, setSelectedChain] = useState("");

  const token = tokenEntries.find((t) => t.denom === selectedDenom) ?? defaultToken;

  const handleDestTypeChange = useCallback((val: DestinationType) => {
    setDestType(val);
    setAddress("");
    setSelectedAccount("");
    setSelectedChain("");
  }, []);

  const handleMax = useCallback(() => {
    setAmount(token.humanBalance);
  }, [token]);

  const handleTokenChange = useCallback((t: TokenEntry) => {
    setSelectedDenom(t.denom);
  }, []);

  const amountDecimal = amount ? Decimal(amount || "0") : Decimal("0");
  const usdValue = getPrice(amount || "0", token.denom);

  const hasTarget =
    (destType === "address" && address.length > 0) ||
    (destType === "account" && selectedAccount.length > 0) ||
    (destType === "bridge" && selectedChain.length > 0);

  const canReview = isConnected && amountDecimal.gt(Decimal("0")) && hasTarget;

  const { mutateAsync: submitSend, isPending } = useSubmitTx<
    void,
    Error,
    { amount: string; recipient: string }
  >({
    submission: { success: "Transfer sent successfully" },
    mutation: {
      mutationFn: async ({ amount: sendAmount, recipient }) => {
        if (!signingClient) throw new Error("Signing client not available");
        if (!account) throw new Error("No active account");

        const parsedAmount = parseUnits(sendAmount, token.decimals);

        await signingClient.transfer({
          transfer: {
            [recipient]: {
              [token.denom]: parsedAmount.toString(),
            },
          },
          sender: account.address as Address,
        });
      },
      onSuccess: () => {
        setAmount("");
        setAddress("");
        setSelectedAccount("");
      },
    },
  });

  const handleReview = useCallback(() => {
    const recipient =
      destType === "address" ? address : destType === "account" ? selectedAccount : "";
    if (!recipient || !amount) return;
    submitSend({ amount, recipient });
  }, [destType, address, selectedAccount, amount, submitSend]);

  const totalBalanceDisplay = useMemo(() => {
    const price = getPrice(token.humanBalance, token.denom);
    return formatDisplay(String(price), 2);
  }, [token, getPrice]);

  return (
    <View className="flex flex-col gap-3.5">
      {/* FROM */}
      <View className="flex flex-col gap-1.5">
        <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">From</Text>
        <View
          className={twMerge(
            "flex flex-row items-center gap-2.5",
            "px-3 py-2.5",
            "bg-bg-sunk border border-border-subtle rounded-field",
          )}
        >
          <AccountAvatar letter={String(account?.index ?? 0)} />
          <View className="flex-1 flex flex-col gap-0.5">
            <Text className="text-fg-primary text-sm font-medium font-display">
              Main {"\u00B7"} Spot
            </Text>
            <Text className="text-fg-tertiary text-xs font-mono">
              ${totalBalanceDisplay} available
            </Text>
          </View>
        </View>
      </View>

      {/* TO */}
      <View className="flex flex-col gap-1.5">
        <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">To</Text>
        <View className="flex flex-row gap-1.5 p-1 bg-bg-sunk rounded-field">
          {DEST_TABS.map((tab) => (
            <DestTabButton
              key={tab.value}
              tab={tab}
              isActive={destType === tab.value}
              onPress={() => handleDestTypeChange(tab.value)}
            />
          ))}
        </View>
      </View>

      {/* Network info */}
      <Text className="text-xs text-fg-tertiary px-0.5">{NETWORK_INFO[destType]}</Text>

      {/* Destination input */}
      <View>
        {destType === "address" && <AddressDestination value={address} onChangeText={setAddress} />}
        {destType === "account" && (
          <AccountDestination
            value={selectedAccount}
            onChange={setSelectedAccount}
            accounts={accounts ?? []}
            currentAddress={account?.address ?? ""}
          />
        )}
        {destType === "bridge" && (
          <BridgeDestination value={selectedChain} onChange={setSelectedChain} />
        )}
      </View>

      {/* AMOUNT */}
      <View className="flex flex-col gap-1.5">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
            Amount
          </Text>
          <Pressable onPress={handleMax} className="flex flex-row items-center gap-1">
            <Text className="text-xs text-fg-tertiary font-mono">Max</Text>
            <Text className="text-xs text-fg-secondary font-mono">
              {formatDisplay(token.humanBalance, 4)} {token.symbol}
            </Text>
          </Pressable>
        </View>
        <View
          className={twMerge(
            "flex flex-row items-center gap-2.5",
            "px-3.5 py-3",
            "bg-bg-surface border border-border-strong rounded-field",
            "focus-within:border-fg-primary focus-within:bg-bg-elev",
            "transition-[border-color,background] duration-150 ease-[var(--ease)]",
          )}
        >
          <TextInput
            value={amount}
            onChangeText={setAmount}
            placeholder="0.00"
            placeholderTextColor="var(--fg-quaternary)"
            inputMode="decimal"
            editable={!isPending}
            className="flex-1 min-w-0 bg-transparent border-0 outline-none font-display text-[24px] font-medium text-fg-primary tabular-nums"
          />
          <TokenPicker value={token} onChange={handleTokenChange} tokens={tokenEntries} />
        </View>
        <View className="flex flex-row items-center justify-between px-0.5">
          <Text className="text-xs text-fg-tertiary font-mono">
            {"\u2248"} ${formatDisplay(String(usdValue), 2)}
          </Text>
          <Text className="text-xs text-fg-tertiary font-mono">
            Available: {formatDisplay(token.humanBalance, 4)} {token.symbol}
          </Text>
        </View>
      </View>

      {/* Submit */}
      <Button
        variant="primary"
        size="lg"
        className="w-full"
        disabled={!canReview || isPending}
        onPress={handleReview}
      >
        <Text className={twMerge("font-semibold text-base", "text-btn-primary-fg")}>
          {isPending ? "Sending..." : "Review send"}
        </Text>
      </Button>
    </View>
  );
}
