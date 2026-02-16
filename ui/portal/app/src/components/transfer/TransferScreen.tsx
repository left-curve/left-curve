import { Modals, useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useBalances,
  useConfig,
  usePublicClient,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useMemo, useState } from "react";
import { Picker } from "@react-native-picker/picker";
import { View, TextInput } from "react-native";

import { isValidAddress } from "@left-curve/dango";
import { capitalize, parseUnits, wait, withResolvers } from "@left-curve/dango/utils";
import { normalizeTransferAction } from "~/features/transfer/params";
import {
  Button,
  CoinIcon,
  GlobalText,
  IconQR,
  MobileTitle,
  ShadowContainer,
  Tabs,
  TextCopy,
  TruncateText,
} from "~/components/foundation";
import { WarningTransferAccounts } from "./WarningTransferAccounts";

import type { Address } from "@left-curve/dango/types";

export const TransferScreen: React.FC = () => {
  const router = useRouter();
  const { showModal } = useApp();
  const params = useLocalSearchParams<{ action?: string | string[] }>();

  const queryClient = useQueryClient();

  const { account, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();
  const publicClient = usePublicClient();

  const [selectedDenom, setSelectedDenom] = useState("bridge/usdc");
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState("0");

  const normalizedAction = normalizeTransferAction(params, isConnected);

  useEffect(() => {
    if (!normalizedAction.changed) return;
    router.setParams({ action: normalizedAction.action });
  }, [normalizedAction.action, normalizedAction.changed, router]);

  const setAction = (nextAction: string) => {
    router.setParams({ action: nextAction === "receive" ? "receive" : "send" });
  };

  const { refetch: refreshBalances, data: balances = {} } = useBalances({
    address: account?.address,
  });

  const isValid20HexAddress = isValidAddress(address || "");

  const { data: doesUserExist = false, isLoading: isCheckingAddress } = useQuery({
    enabled: !!address.length,
    queryKey: ["transfer", address],
    queryFn: async ({ signal }) => {
      await wait(450);
      if (signal.aborted || !isValid20HexAddress) return false;

      const accountInfo = await publicClient.getAccountInfo({
        address: address as Address,
      });

      return !!accountInfo;
    },
  });

  const showAddressWarning =
    !isCheckingAddress &&
    normalizedAction.action === "send" &&
    !!address &&
    isValid20HexAddress &&
    !doesUserExist;

  const selectedCoin = coins.byDenom[selectedDenom] || Object.values(coins.byDenom)[0];

  useEffect(() => {
    if (selectedCoin?.denom) return;
    const defaultCoin = Object.values(coins.byDenom)[0];
    if (defaultCoin) setSelectedDenom(defaultCoin.denom);
  }, [coins.byDenom, selectedCoin]);

  const availableDenoms = useMemo(() => {
    if (!isConnected) return [selectedCoin.denom];
    return Object.keys({ ...balances, "bridge/usdc": "" });
  }, [balances, isConnected, selectedCoin.denom]);

  const { mutateAsync: onSubmit, isPending } = useSubmitTx<void, Error, { amount: string; address: string }>(
    {
      submission: {
        success: m["sendAndReceive.sendSuccessfully"](),
        error: m["transfer.error.description"](),
      },
      mutation: {
        mutationFn: async ({ address: to, amount: sendAmount }, { abort }) => {
          if (!signingClient) throw new Error("error: no signing client");

          const parsedAmount = parseUnits(sendAmount, selectedCoin.decimals);

          const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers<void>();

          showModal(Modals.ConfirmSend, {
            amount: parsedAmount,
            denom: selectedDenom,
            to,
            confirmSend,
            rejectSend,
          });

          await promise.catch(abort);

          await signingClient.transfer({
            transfer: {
              [to]: {
                [selectedCoin.denom]: parsedAmount.toString(),
              },
            },
            sender: account?.address as Address,
          });
        },
        onSuccess: () => {
          setAddress("");
          setAmount("0");
          refreshBalances();
          queryClient.invalidateQueries({ queryKey: ["quests", account?.username] });
        },
      },
    },
  );

  const canSubmit =
    isConnected &&
    !!amount &&
    Number(amount) > 0 &&
    !isPending &&
    isValid20HexAddress &&
    !showAddressWarning;

  return (
    <View className="flex-1 bg-surface-primary-rice px-4 pt-6 gap-4">
      <MobileTitle title={m["sendAndReceive.title"]()} />

      <Tabs
        selectedTab={normalizedAction.action}
        keys={isConnected ? ["send", "receive"] : ["send"]}
        fullWidth
        onTabChange={setAction}
      />

      {normalizedAction.action === "send" ? (
        <View className="gap-4">
          <ShadowContainer borderRadius={12}>
            <View className="rounded-xl bg-surface-secondary-rice p-3 gap-3">
              <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
                {m["sendAndReceive.sending"]()}
              </GlobalText>
              <TextInput
                value={amount}
                keyboardType="decimal-pad"
                onChangeText={(v) => setAmount(v || "0")}
                className="text-ink-secondary-700 diatype-lg-medium"
              />
              <Picker selectedValue={selectedDenom} onValueChange={setSelectedDenom}>
                {availableDenoms
                  .filter((denom) => !!coins.byDenom[denom])
                  .map((denom) => {
                    const coin = coins.byDenom[denom];
                    return (
                      <Picker.Item
                        key={`transfer-coin-${coin.denom}`}
                        label={`${coin.symbol}`}
                        value={coin.denom}
                      />
                    );
                  })}
              </Picker>
            </View>
          </ShadowContainer>

          <ShadowContainer borderRadius={12}>
            <View className="rounded-xl bg-surface-secondary-rice p-3 gap-2">
              <GlobalText className="exposure-sm-italic text-ink-tertiary-500">
                {m["sendAndReceive.to"]()}
              </GlobalText>
              <TextInput
                autoCapitalize="none"
                autoCorrect={false}
                value={address}
                onChangeText={(value) => setAddress(value.toLowerCase().replace(/[^a-z0-9_]/g, ""))}
                placeholder={m["sendAndReceive.inputPlaceholder"]()}
                className="text-ink-secondary-700 diatype-m-medium"
              />
            </View>
          </ShadowContainer>

          {showAddressWarning ? <WarningTransferAccounts variant="send" /> : null}

          <Button isLoading={isPending} isDisabled={!canSubmit} onPress={() => onSubmit({ amount, address })}>
            {m["common.send"]()}
          </Button>
        </View>
      ) : (
        <View className="gap-4">
          <WarningTransferAccounts variant="receive" />

          <ShadowContainer borderRadius={12}>
            <View className="rounded-xl bg-surface-secondary-rice p-4 gap-3 items-center">
              <View className="items-center gap-1">
                <GlobalText className="h3-bold">
                  {`${capitalize((account?.type as string) || "") || "Spot"} Account #${account?.index || 1}`}
                </GlobalText>
                <View className="flex-row items-center gap-1">
                  <TruncateText
                    className="diatype-sm-medium text-ink-tertiary-500"
                    text={account?.address}
                  />
                  <TextCopy copyText={account?.address} className="text-ink-tertiary-500" />
                </View>
              </View>

              <View className="w-[220px] h-[220px] rounded-xl bg-surface-primary-rice border border-outline-secondary-gray items-center justify-center">
                <IconQR className="w-14 h-14 text-ink-tertiary-500" />
                <GlobalText className="diatype-sm-regular text-ink-tertiary-500 mt-2">
                  Scan to receive
                </GlobalText>
              </View>
            </View>
          </ShadowContainer>
        </View>
      )}

      <View className="mt-auto mb-2 flex-row items-center justify-center gap-2">
        <CoinIcon symbol={selectedCoin.symbol} size={18} />
        <GlobalText className="diatype-sm-regular text-ink-tertiary-500">
          Balance: {balances[selectedCoin.denom] || "0"} {selectedCoin.symbol}
        </GlobalText>
      </View>
    </View>
  );
};
