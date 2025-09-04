import { useTheme } from "~/hooks/useTheme";

import { View, Pressable } from "react-native";
import { GlobalText } from "../foundation/GlobalText";
import { Badge } from "../foundation/Badge";
import TruncateText from "../foundation/TruncateText";
import { LinearGradient } from "expo-linear-gradient";

import { twMerge } from "@left-curve/foundation";

import type React from "react";
import type { Account, AccountTypes } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";
import { TextCopy } from "../foundation/TextCopy";

export enum AccountType {
  Spot = "spot",
  Margin = "margin",
  Multi = "multi",
}

const CARD_GRADIENTS = {
  [AccountType.Multi]: {
    light: ["#F6F6FB", "#DDDCEE", "#F6F6FB"],
    dark: ["#373634", "#6E6D77", "#373634"],
  },
  [AccountType.Margin]: {
    light: ["#F8F9EF", "#EFF0C3", "#F8F9EF"],
    dark: ["#373634", "#666654", "#373634"],
  },
  [AccountType.Spot]: {
    light: ["#FFFBF5", "#F9E2E2", "#FFFBF4"],
    dark: ["#494443", "#584D4E", "#322F2F"],
  },
} as const;

export const CARD_IMAGES = {
  dog: require("@left-curve/foundation/images/characters/dog.svg"),
  puppy: require("@left-curve/foundation/images/characters/puppy.svg"),
  froggo: require("@left-curve/foundation/images/characters/froggo.svg"),
};

export const AccountCardOptions = {
  [AccountType.Spot]: {
    text: "Spot",
    badge: "blue" as const,
    Image: CARD_IMAGES.dog.default,
    imageClassName: "opacity-60 right-[-6rem] bottom-[-20rem] scale-x-[-1] w-[17rem]",
  },
  [AccountType.Multi]: {
    text: "Multisig",
    badge: "green" as const,
    Image: CARD_IMAGES.puppy.default,
    imageClassName: "opacity-50 right-[-3rem] bottom-[-25rem] w-[21rem]",
  },
  [AccountType.Margin]: {
    text: "Margin",
    badge: "red" as const,
    bgColor: "bg-account-card-green",
    Image: CARD_IMAGES.froggo.default,
    imageClassName: "opacity-60 right-[-4rem] bottom-[-27rem] w-[19rem]",
  },
} as const;

type AccountCardContainerProps = {
  variant: AccountTypes;
};

export const AccountCardContainer: React.FC<PropsWithChildren<AccountCardContainerProps>> = ({
  variant,
  children,
}) => {
  const { theme } = useTheme();
  const colors = CARD_GRADIENTS[variant][theme];

  return (
    <View className="shadow relative overflow-hidden rounded-xl">
      <LinearGradient
        colors={colors}
        start={{ x: 0, y: 0.5 }}
        end={{ x: 1, y: 0.5 }}
        className="rounded-xl text-secondary-700"
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

export const AccountCard: React.FC<AccountCardProps> = ({
  account,
  balance,
  balanceChange,
  onTriggerAction,
  isSelectorActive,
}) => {
  const { address, type, index } = account;
  const name = `${type} #${index}`;

  const { badge, Image, imageClassName, text } = AccountCardOptions[type];

  return (
    <AccountCardContainer variant={type}>
      <Image className={twMerge("absolute", imageClassName)} />

      <View className="flex flex-col relative z-10">
        <View className="flex-row gap-1 items-center">
          <GlobalText className="exposure-l-italic capitalize">{name}</GlobalText>
          <Badge text={text} color={badge} size="s" />
        </View>

        <View className="flex-row gap-1 items-center">
          <TruncateText
            text={address}
            className="diatype-xs-medium text-tertiary-500"
            start={4}
            end={4}
          />
          <TextCopy copyText={address} className="w-4 h-4 text-tertiary-500" />
        </View>
      </View>

      {type === AccountType.Margin ? (
        <View>Borrowbar</View>
      ) : (
        <View className="flex-row gap-2 items-center relative z-10">
          <GlobalText className="h2-medium">${balance}</GlobalText>
          {!!balanceChange && (
            <GlobalText className="text-sm font-bold text-status-success">
              {balanceChange}
            </GlobalText>
          )}
        </View>
      )}
    </AccountCardContainer>
  );
};

type AccountCardPreviewProps = {
  account: Account;
  onAccountSelect: (account: Account) => void;
};

const Preview: React.FC<AccountCardPreviewProps> = ({ account, onAccountSelect }) => {
  const { address, index } = account;
  const type = account.type as AccountTypes;
  const name = `${type} #${index}`;

  const { badge, text } = AccountCardOptions[type];

  const totalBalance = 120;

  return (
    <Pressable
      className={twMerge(
        "shadow-account-card w-full max-w-[360px] md:max-w-[328px] lg:min-w-[328px] -mb-[99px] flex-shrink-0 h-[160px] relative overflow-hidden rounded-xl flex flex-col justify-between p-4 text-secondary-700",
      )}
      onPress={() => onAccountSelect(account)}
      accessibilityRole="button"
    >
      <View className="flex-row items-start justify-between relative z-10">
        <View className="flex-col">
          <View className="flex-row gap-1 items-center">
            <GlobalText className="exposure-m-italic capitalize text-tertiary-500">
              {name}
            </GlobalText>
          </View>

          <View className="flex-row gap-1 items-center">
            <TruncateText
              text={address}
              className="diatype-xs-medium text-tertiary-500"
              start={4}
              end={4}
            />
            <TextCopy copyText={address} className="w-4 h-4 text-tertiary-500" />
          </View>
        </View>

        <View className="flex-col gap-1 items-end">
          <GlobalText className="diatype-m-bold text-tertiary-500">{totalBalance}</GlobalText>
          <Badge text={text} color={badge} size="s" />
        </View>
      </View>
    </Pressable>
  );
};

export const AccountCardRN = Object.assign(AccountCard, { Preview });
