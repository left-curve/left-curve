import { useState, useCallback } from "react";
import { View, Text, Pressable, TextInput } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Decimal, formatUnits, parseUnits, truncateDec } from "@left-curve/dango/utils";
import {
  useAccount,
  useBalances,
  useConfig,
  useSigningClient,
  useSubmitTx,
  usePerpsUserStateExtended,
  perpsUserStateExtendedStore,
} from "@left-curve/store";
import type { Address } from "@left-curve/dango/types";
import { Button, IconButton } from "../components";

type Direction = "toPerps" | "toSpot";

function formatDisplay(value: string, decimals = 2): string {
  return Number(Number(value).toFixed(decimals)).toLocaleString("en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

function AccountAvatar({
  letter,
  kind,
}: {
  readonly letter: string;
  readonly kind: "spot" | "perps";
}) {
  return (
    <View className="relative">
      <View className="w-7 h-7 rounded-field bg-up-bg items-center justify-center">
        <Text className="text-up font-mono font-semibold text-[11px]">{letter}</Text>
      </View>
      <View
        className={twMerge(
          "absolute -right-1 -bottom-1",
          "px-[5px] py-[2px] rounded-chip",
          "border border-border-subtle",
          kind === "perps" ? "bg-accent" : "bg-bg-surface",
        )}
      >
        <Text
          className={twMerge(
            "text-[8px] font-bold font-display tracking-caps uppercase leading-none",
            kind === "perps" ? "text-accent-fg" : "text-fg-secondary",
          )}
        >
          {kind === "perps" ? "PRP" : "SPT"}
        </Text>
      </View>
    </View>
  );
}

function BalanceRow({
  label,
  accountLetter,
  accountName,
  kind,
  balance,
}: {
  readonly label: string;
  readonly accountLetter: string;
  readonly accountName: string;
  readonly kind: "spot" | "perps";
  readonly balance: string;
}) {
  const unit = kind === "spot" ? "USDC" : "USD";
  return (
    <View className="flex flex-row items-center gap-2.5">
      <Text className="text-[10px] text-fg-tertiary tracking-caps uppercase w-9">{label}</Text>
      <AccountAvatar letter={accountLetter} kind={kind} />
      <View className="flex-1 flex flex-col gap-0.5">
        <Text className="text-fg-primary text-[12px] font-medium font-display">
          {accountName} {"\u00B7"} {kind === "spot" ? "Spot" : "Perps"}
        </Text>
        <Text className="text-fg-tertiary text-[11px] font-mono">
          {kind === "spot" ? "" : "$"}
          {formatDisplay(balance)} {unit}
        </Text>
      </View>
    </View>
  );
}

export function SpotPerpsTab() {
  const { account, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: balances = {} } = useBalances({ address: account?.address as Address });
  const { data: signingClient } = useSigningClient();

  usePerpsUserStateExtended();
  const availableMargin = perpsUserStateExtendedStore((s) => s.availableMargin);

  const usdcCoin = coins.byDenom["bridge/usdc"];

  const spotUsdcRaw = balances["bridge/usdc"] || "0";
  const spotUsdcHuman = usdcCoin ? formatUnits(spotUsdcRaw, usdcCoin.decimals) : "0";
  const perpsUsdHuman = availableMargin || "0";

  const [direction, setDirection] = useState<Direction>("toPerps");
  const [amount, setAmount] = useState("");

  const accountLetter = String(account?.index ?? 0);
  const accountName = `Account ${account?.index ?? 0}`;
  const fromKind = direction === "toPerps" ? "spot" : "perps";
  const toKind = direction === "toPerps" ? "perps" : "spot";
  const maxBalance = direction === "toPerps" ? spotUsdcHuman : perpsUsdHuman;
  const fromUnit = direction === "toPerps" ? "USDC" : "USD";
  const toUnit = direction === "toPerps" ? "USD" : "USDC";
  const fromBalance = direction === "toPerps" ? spotUsdcHuman : perpsUsdHuman;
  const toBalance = direction === "toPerps" ? perpsUsdHuman : spotUsdcHuman;

  const handleFlip = useCallback(() => {
    setDirection((prev) => {
      const next = prev === "toPerps" ? "toSpot" : "toPerps";
      const nextMax = next === "toPerps" ? spotUsdcHuman : perpsUsdHuman;
      if (amount && Decimal(amount).gt(Decimal(nextMax))) {
        setAmount(nextMax);
      }
      return next;
    });
  }, [spotUsdcHuman, perpsUsdHuman, amount]);

  const handleMax = useCallback(() => {
    setAmount(maxBalance);
  }, [maxBalance]);

  const amountDecimal = amount ? Decimal(amount || "0") : Decimal("0");
  const canConfirm =
    isConnected && amountDecimal.gt(Decimal("0")) && amountDecimal.lte(Decimal(maxBalance));

  const isSpotToPerp = direction === "toPerps";

  const { mutateAsync: submitTransfer, isPending } = useSubmitTx<void, Error, { amount: string }>({
    submission: {
      success: isSpotToPerp ? "Deposit to Perps successful" : "Withdraw to Spot successful",
    },
    mutation: {
      mutationFn: async ({ amount: transferAmount }) => {
        if (!signingClient) throw new Error("Signing client not available");
        if (!account) throw new Error("No active account");

        const sender = account.address as Address;

        if (isSpotToPerp) {
          const parsedAmount = parseUnits(transferAmount, usdcCoin.decimals);
          await signingClient.depositMargin({
            sender,
            amount: parsedAmount.toString(),
          });
        } else {
          await signingClient.withdrawMargin({
            sender,
            amount: truncateDec(transferAmount),
          });
        }
      },
      onSuccess: () => {
        setAmount("");
      },
    },
  });

  const handleConfirm = useCallback(() => {
    if (!amount || !canConfirm) return;
    submitTransfer({ amount });
  }, [amount, canConfirm, submitTransfer]);

  return (
    <View className="flex flex-col gap-3">
      <View
        className={twMerge(
          "flex flex-col gap-2.5 mt-1",
          "p-3.5",
          "bg-bg-sunk border border-border-subtle rounded-field",
        )}
      >
        <BalanceRow
          label="From"
          accountLetter={accountLetter}
          accountName={accountName}
          kind={fromKind}
          balance={fromBalance}
        />

        <View className="flex flex-row items-center justify-center">
          <IconButton
            shape="circle"
            onPress={handleFlip}
            className="border-border-subtle bg-bg-surface"
          >
            <Text className="text-fg-tertiary text-[12px]">{"\u2195"}</Text>
          </IconButton>
        </View>

        <BalanceRow
          label="To"
          accountLetter={accountLetter}
          accountName={accountName}
          kind={toKind}
          balance={toBalance}
        />
      </View>

      <View className="flex flex-col gap-1.5">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-[11px] text-fg-tertiary tracking-wide uppercase font-semibold">
            Amount
          </Text>
          <Pressable onPress={handleMax} className="flex flex-row items-center gap-1">
            <Text className="text-[11px] text-fg-tertiary font-mono">Max</Text>
            <Text className="text-[11px] text-fg-secondary font-mono">
              {formatDisplay(maxBalance)}
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
          <View className="flex flex-row items-center gap-1.5 px-2.5 py-1.5 bg-bg-tint rounded-field">
            <Text className="text-fg-primary text-[12px] font-medium font-display">{fromUnit}</Text>
            <Text className="text-fg-tertiary text-[10px]">locked</Text>
          </View>
        </View>
        <Text className="text-[11px] text-fg-tertiary px-0.5">
          Only {fromUnit} can move between Spot and Perps.
        </Text>
      </View>

      <View
        className={twMerge(
          "flex flex-row items-center gap-2",
          "px-3 py-2.5",
          "bg-accent-bg rounded-field",
        )}
      >
        <Text className="text-fg-tertiary text-[11px]">{"\u24D8"}</Text>
        <Text className="text-fg-secondary text-[12px] flex-1 leading-relaxed">
          {direction === "toPerps" ? (
            <>
              {fromUnit} will appear as{" "}
              <Text className="text-fg-primary font-medium">{toUnit}</Text> in your Perps balance.
              Instant {"\u00B7"} no fee.
            </>
          ) : (
            <>
              {fromUnit} will appear as{" "}
              <Text className="text-fg-primary font-medium">{toUnit}</Text> in your Spot balance.
              Instant {"\u00B7"} no fee.
            </>
          )}
        </Text>
      </View>

      <Button
        variant="primary"
        size="lg"
        className="w-full"
        disabled={!canConfirm || isPending}
        onPress={handleConfirm}
      >
        <Text className="font-semibold text-[14px] text-btn-primary-fg">
          {isPending
            ? "Processing..."
            : `Confirm ${direction === "toPerps" ? "deposit" : "withdraw"}`}
        </Text>
      </Button>
    </View>
  );
}
