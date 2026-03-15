import { useTheme } from "~/hooks/useTheme";

import { View, Pressable } from "react-native";
import { GlobalText } from "../foundation/GlobalText";
import { Badge } from "../foundation/Badge";
import TruncateText from "../foundation/TruncateText";
import { LinearGradient } from "expo-linear-gradient";

import { twMerge } from "@left-curve/foundation";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import type React from "react";
import type { Account } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";
import { TextCopy } from "../foundation/TextCopy";

const CARD_GRADIENT = {
  light: ["#FFFBF5", "#F9E2E2", "#FFFBF4"],
  dark: ["#494443", "#584D4E", "#322F2F"],
} as const;

export const CARD_IMAGES = {
  dog: require("@left-curve/foundation/images/characters/dog.svg"),
  puppy: require("@left-curve/foundation/images/characters/puppy.svg"),
  froggo: require("@left-curve/foundation/images/characters/froggo.svg"),
};

const accountCardStyle = {
  badge: "blue" as const,
  Image: CARD_IMAGES.dog.default,
  imageClassName: "opacity-60 right-[-6rem] bottom-[-20rem] scale-x-[-1] w-[17rem]",
} as const;

export const AccountCardContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const { theme } = useTheme();
  const colors = CARD_GRADIENT[theme];

  return (
    <View className="shadow relative overflow-hidden rounded-xl">
      <LinearGradient
        colors={colors}
        start={{ x: 0, y: 0.5 }}
        end={{ x: 1, y: 0.5 }}
        className="rounded-xl text-ink-secondary-700"
      >
        <View
          key="content"
          className="w-full max-w-[360px] h-[13rem] p-4 flex flex-col justify-between relative"
        >
          {children}
        </View>
      </LinearGradient>
    </View>
  );
};

type AccountCardProps = {
  account: Account;
  balance: string;
  balanceChange?: string;
  isSelectorActive?: boolean;
  onTriggerAction?: () => void;
};

export const AccountCard: React.FC<AccountCardProps> = ({ account, balance, balanceChange }) => {
  const { address, index } = account;
  const name = `Account #${index}`;
  const client = usePublicClient();

  const { data: status } = useQuery({
    queryKey: ["accountStatus", address],
    queryFn: () => client.getAccountStatus({ address }),
  });

  const isActive = status === "active";

  const { Image, imageClassName } = accountCardStyle;

  return (
    <AccountCardContainer>
      <Image className={twMerge("absolute", imageClassName)} />

      <View className="flex flex-col relative z-10">
        <View className="flex-row gap-1 items-center">
          <GlobalText className="exposure-l-italic capitalize">{name}</GlobalText>
          <Badge
            text={isActive ? "Active" : "Inactive"}
            color={isActive ? "blue" : "red"}
            size="s"
          />
        </View>

        <View className="flex-row gap-1 items-center">
          <TruncateText
            text={address}
            className="diatype-xs-medium text-ink-tertiary-500"
            start={4}
            end={4}
          />
          <TextCopy copyText={address} className="w-4 h-4 text-ink-tertiary-500" />
        </View>
      </View>

      <View className="flex-row gap-2 items-center relative z-10">
        <GlobalText className="h2-medium">${balance}</GlobalText>
        {!!balanceChange && (
          <GlobalText className="text-sm font-bold text-status-success">{balanceChange}</GlobalText>
        )}
      </View>
    </AccountCardContainer>
  );
};

type AccountCardPreviewProps = {
  account: Account;
  onAccountSelect: (account: Account) => void;
};

const Preview: React.FC<AccountCardPreviewProps> = ({ account, onAccountSelect }) => {
  const { address, index } = account;
  const name = `Account #${index}`;

  const { badge } = accountCardStyle;

  const totalBalance = 120;

  return (
    <Pressable
      className={twMerge(
        "shadow-account-card w-full max-w-[360px] md:max-w-[328px] lg:min-w-[328px] -mb-[99px] flex-shrink-0 h-[160px] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 text-ink-secondary-700",
      )}
      onPress={() => onAccountSelect(account)}
      accessibilityRole="button"
    >
      <View className="flex-row items-start justify-between relative z-10">
        <View className="flex-col">
          <View className="flex-row gap-1 items-center">
            <GlobalText className="exposure-m-italic capitalize text-ink-tertiary-500">
              {name}
            </GlobalText>
          </View>

          <View className="flex-row gap-1 items-center">
            <TruncateText
              text={address}
              className="diatype-xs-medium text-ink-tertiary-500"
              start={4}
              end={4}
            />
            <TextCopy copyText={address} className="w-4 h-4 text-ink-tertiary-500" />
          </View>
        </View>

        <View className="flex-col gap-1 items-end">
          <GlobalText className="diatype-m-bold text-ink-tertiary-500">{totalBalance}</GlobalText>
          <Badge text="Active" color={badge} size="s" />
        </View>
      </View>
    </Pressable>
  );
};

export const AccountCardRN = Object.assign(AccountCard, { Preview });
