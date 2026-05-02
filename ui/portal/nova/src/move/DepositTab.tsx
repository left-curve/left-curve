import { useState, useCallback, useMemo } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { useAccount, useConfig } from "@left-curve/store";

type DepositMode = "onchain" | "bridge";

type DepositModeConfig = {
  readonly value: DepositMode;
  readonly label: string;
  readonly icon: string;
  readonly hint: string;
};

const DEPOSIT_MODES: readonly DepositModeConfig[] = [
  { value: "onchain", label: "On-chain", icon: "\u2B07", hint: "Dango network" },
  { value: "bridge", label: "Bridge", icon: "\u2197", hint: "Other chain" },
];

type BridgeNetwork = {
  readonly id: string;
  readonly name: string;
  readonly shortName: string;
  readonly assets: string;
  readonly time: string;
};

const BRIDGE_NETWORKS: readonly BridgeNetwork[] = [
  { id: "1", name: "Ethereum", shortName: "Et", assets: "USDC, USDT", time: "~3 min" },
  { id: "8453", name: "Base", shortName: "Ba", assets: "USDC, WETH", time: "~5 min" },
  { id: "42161", name: "Arbitrum", shortName: "Ar", assets: "USDC, USDT", time: "~5 min" },
];

const SUPPORTED_ASSETS = "USDC, USDT, WETH" as const;

function AccountAvatar({ letter }: { readonly letter: string }) {
  return (
    <View className="w-7 h-7 rounded-field bg-up-bg items-center justify-center">
      <Text className="text-up font-mono font-semibold text-xs">{letter}</Text>
    </View>
  );
}

function ModeTabButton({
  tab,
  isActive,
  onPress,
}: {
  readonly tab: DepositModeConfig;
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

function QRCodePlaceholder({ address }: { readonly address: string }) {
  const cells = useMemo(() => {
    const seed = address.length > 0 ? address : "placeholder";
    return Array.from({ length: 21 * 21 }, (_, i) => {
      const x = i % 21;
      const y = Math.floor(i / 21);
      const inAnchor = (ax: number, ay: number) => x >= ax && x < ax + 7 && y >= ay && y < ay + 7;
      const inRing = (ax: number, ay: number) =>
        inAnchor(ax, ay) && (x === ax || x === ax + 6 || y === ay || y === ay + 6);
      const inCore = (ax: number, ay: number) =>
        x >= ax + 2 && x < ax + 5 && y >= ay + 2 && y < ay + 5;
      const isAnchor =
        inRing(0, 0) ||
        inCore(0, 0) ||
        inRing(14, 0) ||
        inCore(14, 0) ||
        inRing(0, 14) ||
        inCore(0, 14);
      const hash = seed.charCodeAt((x + y * 3) % seed.length) || 0;
      const noise = !isAnchor && (x * 7919 + y * 7547 + hash) % 5 < 2;
      return { key: `${x}-${y}`, filled: isAnchor || noise };
    });
  }, [address]);

  return (
    <View
      className="w-[180px] h-[180px] bg-white border border-border-subtle rounded-card p-3 self-center"
      style={{
        display: "grid" as never,
        gridTemplateColumns: "repeat(21, 1fr)",
        gridTemplateRows: "repeat(21, 1fr)",
        gap: 1,
      }}
    >
      {cells.map((cell) => (
        <View key={cell.key} style={{ backgroundColor: cell.filled ? "black" : "transparent" }} />
      ))}
    </View>
  );
}

function AddressCard({
  address,
  label,
  onCopy,
  copied,
}: {
  readonly address: string;
  readonly label: string;
  readonly onCopy: () => void;
  readonly copied: boolean;
}) {
  return (
    <View className="self-stretch flex flex-col gap-1.5">
      <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
        {label}
      </Text>
      <View
        className={twMerge(
          "flex flex-row items-center gap-2.5",
          "px-3 py-2.5",
          "bg-bg-sunk border border-border-subtle rounded-field",
        )}
      >
        <Text className="flex-1 font-mono text-sm text-fg-primary" numberOfLines={1}>
          {address}
        </Text>
        <Pressable
          onPress={onCopy}
          className={twMerge(
            "px-2.5 py-1 rounded-field",
            "transition-[background,color] duration-150 ease-[var(--ease)]",
            copied ? "bg-up-bg" : "bg-bg-tint hover:bg-bg-surface",
          )}
        >
          <Text className={twMerge("text-xs font-medium", copied ? "text-up" : "text-fg-tertiary")}>
            {copied ? "\u2713 Copied" : "Copy"}
          </Text>
        </Pressable>
      </View>
    </View>
  );
}

function OnchainDeposit({
  depositAddress,
  networkName,
  accountLabel,
  onCopy,
  copied,
}: {
  readonly depositAddress: string;
  readonly networkName: string;
  readonly accountLabel: string;
  readonly onCopy: () => void;
  readonly copied: boolean;
}) {
  return (
    <View className="flex flex-col gap-3.5">
      <Text className="text-xs text-fg-tertiary px-0.5">
        On-chain {"\u00B7"} {networkName} network {"\u00B7"} direct deposit
      </Text>

      <QRCodePlaceholder address={depositAddress} />

      <AddressCard
        address={depositAddress}
        label="Your deposit address"
        onCopy={onCopy}
        copied={copied}
      />

      <View className="self-stretch flex flex-col gap-1.5">
        <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
          Supported assets
        </Text>
        <View
          className={twMerge(
            "flex flex-row flex-wrap gap-1.5",
            "px-3 py-2.5",
            "bg-bg-surface border border-border-subtle rounded-field",
          )}
        >
          {SUPPORTED_ASSETS.split(", ").map((asset) => (
            <View key={asset} className="px-2 py-1 bg-bg-tint rounded-field">
              <Text className="text-xs text-fg-primary font-medium font-mono">{asset}</Text>
            </View>
          ))}
        </View>
      </View>

      <Text className="text-xs text-fg-tertiary leading-relaxed px-0.5">
        Send funds to this address on the{" "}
        <Text className="text-fg-primary font-medium">{networkName}</Text> network. Funds land in{" "}
        <Text className="text-fg-primary font-medium">
          {accountLabel} {"\u00B7"} Spot
        </Text>
        . To use them in Perps, switch to the {"\u201C"}Spot {"\u2194"} Perps{"\u201D"} tab after
        receiving.
      </Text>
    </View>
  );
}

function BridgeDeposit({
  depositAddress,
  accountLabel,
  onCopy,
  copied,
}: {
  readonly depositAddress: string;
  readonly accountLabel: string;
  readonly onCopy: () => void;
  readonly copied: boolean;
}) {
  const [selectedNetwork, setSelectedNetwork] = useState<string>("");

  const activeNetwork = BRIDGE_NETWORKS.find((n) => n.id === selectedNetwork);

  return (
    <View className="flex flex-col gap-3.5">
      <Text className="text-xs text-fg-tertiary px-0.5">
        Bridge {"\u00B7"} deposit from another chain {"\u00B7"} via Hyperlane
      </Text>

      <View className="self-stretch flex flex-col gap-1.5">
        <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
          Source network
        </Text>
        <View style={{ display: "grid" as never, gridTemplateColumns: "1fr 1fr 1fr", gap: 6 }}>
          {BRIDGE_NETWORKS.map((network) => (
            <Pressable
              key={network.id}
              onPress={() => setSelectedNetwork(network.id)}
              className={twMerge(
                "flex flex-col items-center gap-1.5 px-3 py-2.5 rounded-field",
                "border transition-[border-color] duration-150 ease-[var(--ease)]",
                selectedNetwork === network.id
                  ? "border-fg-primary bg-bg-surface"
                  : "border-border-default bg-bg-surface hover:border-border-strong",
              )}
            >
              <View className="w-[26px] h-[26px] rounded-full bg-bg-tint items-center justify-center">
                <Text className="text-fg-secondary text-2xs font-semibold font-mono">
                  {network.shortName}
                </Text>
              </View>
              <Text className="text-fg-primary text-sm font-medium font-display">
                {network.name}
              </Text>
              <Text className="text-fg-tertiary text-2xs">{network.time}</Text>
            </Pressable>
          ))}
        </View>
      </View>

      {activeNetwork ? (
        <>
          <View className="self-stretch flex flex-col gap-1.5">
            <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
              Available on {activeNetwork.name}
            </Text>
            <View
              className={twMerge(
                "flex flex-row flex-wrap gap-1.5",
                "px-3 py-2.5",
                "bg-bg-surface border border-border-subtle rounded-field",
              )}
            >
              {activeNetwork.assets.split(", ").map((asset) => (
                <View key={asset} className="px-2 py-1 bg-bg-tint rounded-field">
                  <Text className="text-xs text-fg-primary font-medium font-mono">{asset}</Text>
                </View>
              ))}
            </View>
          </View>

          <QRCodePlaceholder address={depositAddress} />

          <AddressCard
            address={depositAddress}
            label={`Deposit address (${activeNetwork.name})`}
            onCopy={onCopy}
            copied={copied}
          />

          <Text className="text-xs text-fg-tertiary leading-relaxed px-0.5">
            Send supported tokens from{" "}
            <Text className="text-fg-primary font-medium">{activeNetwork.name}</Text> to this
            address. Funds are bridged via Hyperlane and arrive in{" "}
            <Text className="text-fg-primary font-medium">
              {accountLabel} {"\u00B7"} Spot
            </Text>{" "}
            within {activeNetwork.time}.
          </Text>
        </>
      ) : (
        <View className="flex flex-col items-center gap-2 py-4">
          <Text className="text-fg-tertiary text-sm">
            Select a network above to see your deposit address
          </Text>
        </View>
      )}
    </View>
  );
}

export function DepositTab() {
  const { account, accounts, isConnected } = useAccount();
  const { chain } = useConfig();

  const [mode, setMode] = useState<DepositMode>("onchain");
  const [copied, setCopied] = useState(false);

  const accountList = useMemo(() => {
    if (!accounts) return [];
    return accounts.map((acct) => ({
      address: acct.address,
      index: acct.index,
      label: `Account ${acct.index}`,
      description: acct.address === account?.address ? "Current account" : "Sub-account",
    }));
  }, [accounts, account?.address]);

  const [selectedAddress, setSelectedAddress] = useState(account?.address ?? "");

  const selectedEntry = accountList.find((a) => a.address === selectedAddress) ?? accountList[0];
  const depositAddress = selectedEntry?.address ?? account?.address ?? "";
  const networkName = chain?.name ?? "Dango";
  const accountLabel = selectedEntry?.label ?? "Account";

  const handleCopy = useCallback(() => {
    if (!depositAddress) return;
    navigator.clipboard.writeText(depositAddress).catch(() => {});
    setCopied(true);
    const timeout = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timeout);
  }, [depositAddress]);

  const handleModeChange = useCallback((val: DepositMode) => {
    setMode(val);
    setCopied(false);
  }, []);

  if (!isConnected) {
    return (
      <View className="flex flex-col gap-3.5 items-center py-6">
        <View className="w-10 h-10 rounded-full bg-bg-tint items-center justify-center self-center">
          <Text className="text-fg-tertiary text-lg">{"\u2B07"}</Text>
        </View>
        <Text className="text-fg-tertiary text-sm text-center">
          {mode === "onchain"
            ? "Connect your wallet to view your deposit address"
            : "Connect your wallet to bridge funds"}
        </Text>
        <Text className="text-fg-quaternary text-xs text-center">
          Your Dango address will appear here once connected
        </Text>
      </View>
    );
  }

  return (
    <View className="flex flex-col gap-3.5">
      {/* Mode selector */}
      <View className="flex flex-row gap-1.5 p-1 bg-bg-sunk rounded-field">
        {DEPOSIT_MODES.map((tab) => (
          <ModeTabButton
            key={tab.value}
            tab={tab}
            isActive={mode === tab.value}
            onPress={() => handleModeChange(tab.value)}
          />
        ))}
      </View>

      {/* Account selector */}
      <View className="self-stretch flex flex-col gap-1.5">
        <Text className="text-xs text-fg-tertiary tracking-wide uppercase font-semibold">
          Receive into
        </Text>
        <View className="flex flex-col gap-1.5">
          {accountList.map((entry) => (
            <Pressable
              key={entry.address}
              onPress={() => {
                setSelectedAddress(entry.address);
                setCopied(false);
              }}
              className={twMerge(
                "flex flex-row items-center gap-3 px-3 py-2.5 rounded-field",
                "border transition-[border-color] duration-150 ease-[var(--ease)]",
                selectedAddress === entry.address
                  ? "border-fg-primary bg-bg-surface"
                  : "border-border-default bg-bg-surface hover:border-border-strong",
              )}
            >
              <AccountAvatar letter={String(entry.index)} />
              <View className="flex-1 flex flex-col gap-0.5">
                <Text className="text-fg-primary text-sm font-medium font-display">
                  {entry.label} {"\u00B7"} Spot
                </Text>
                <Text className="text-fg-tertiary text-xs">{entry.description}</Text>
              </View>
              {selectedAddress === entry.address && (
                <Text className="text-fg-primary text-sm">{"\u2713"}</Text>
              )}
            </Pressable>
          ))}
        </View>
      </View>

      {/* Mode content */}
      {mode === "onchain" ? (
        <OnchainDeposit
          depositAddress={depositAddress}
          networkName={networkName}
          accountLabel={accountLabel}
          onCopy={handleCopy}
          copied={copied}
        />
      ) : (
        <BridgeDeposit
          depositAddress={depositAddress}
          accountLabel={accountLabel}
          onCopy={handleCopy}
          copied={copied}
        />
      )}
    </View>
  );
}
