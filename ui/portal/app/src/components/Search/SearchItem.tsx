import { useFavApplets } from "@left-curve/store";

import { MotiView } from "moti";
import { View, Pressable } from "react-native";
import { GlobalText, IconEmptyStar, IconStar, TruncateText } from "../foundation";

import { ASSETS } from "~/constants";
import EmojiTxs from "@left-curve/foundation/images/emojis/simple/txs.svg";
import EmojiBlocks from "@left-curve/foundation/images/emojis/simple/blocks.svg";
import EmojiFactory from "@left-curve/foundation/images/emojis/simple/protrading.svg";

import type React from "react";
import type { Account, Address, ContractInfo } from "@left-curve/dango/types";
import type { AnyCoin, AppletMetadata, WithPrice } from "@left-curve/store/types";
import { AddressVisualizer } from "../foundation/AddressVisualizer";

const Root: React.FC<React.PropsWithChildren> = ({ children }) => <>{children}</>;

type SearchAppletItemProps = AppletMetadata;

const AppletItem: React.FC<SearchAppletItemProps> = (applet) => {
  const { id, title, description } = applet;
  const { favApplets, addFavApplet, removeFavApplet } = useFavApplets();
  const isFav = favApplets.includes(id);
  const AppletImage = ASSETS[applet.id as keyof typeof ASSETS].default;

  const onPressStar = () => {
    if (isFav) removeFavApplet(applet);
    else addFavApplet(applet);
  };

  return (
    <MotiView key={title} className="w-full p-2 rounded-xs">
      <View className="flex flex-row items-center justify-between gap-4">
        <View className="flex-row items-center gap-4 flex-1 basis-0">
          <View className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
            {AppletImage ? <AppletImage width={48} height={48} /> : null}
          </View>

          <View className="flex flex-col max-w-full flex-1">
            <GlobalText className="diatype-lg-medium text-secondary-700">{title}</GlobalText>
            <GlobalText
              className="diatype-m-regular text-tertiary-500"
              numberOfLines={2}
              ellipsizeMode="tail"
            >
              {description}
            </GlobalText>
          </View>
        </View>

        <Pressable
          onPress={onPressStar}
          accessibilityRole="button"
          hitSlop={8}
          className="shrink-0 ml-2"
        >
          {isFav ? (
            <IconStar className="w-6 h-6 text-rice-500" />
          ) : (
            <IconEmptyStar className="w-6 h-6 text-rice-500" />
          )}
        </Pressable>
      </View>
    </MotiView>
  );
};

type SearchAssetProps = WithPrice<AnyCoin>;

const AssetItem: React.FC<SearchAssetProps> = ({ symbol, price }) => {
  return (
    <MotiView key={symbol} className="w-full p-2 min-h-[74px] rounded-xs">
      <View className="flex-row items-start justify-between">
        <View className="flex-row items-start gap-4">
          {/* <Image source={{ uri: logoURI }} style={{ width: 32, height: 32 }} /> */}
          <View className="flex-col gap-1">
            <GlobalText className="diatype-m-bold">{symbol}</GlobalText>
            <GlobalText className="diatype-m-regular text-tertiary-500">{symbol}</GlobalText>
          </View>
        </View>
        <View className="flex-col gap-1">
          <GlobalText className="diatype-sm-bold">${price}</GlobalText>
        </View>
      </View>
    </MotiView>
  );
};

type SearchBlockItemProps = { height: number; hash: string };

const BlockItem: React.FC<SearchBlockItemProps> = ({ height, hash }) => {
  return (
    <MotiView key={height} className="w-full p-2 min-h-[74px] rounded-xs">
      <View className="flex flex-row items-center gap-4">
        <View className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <EmojiBlocks width={48} height={48} />
        </View>
        <View className="flex-col">
          <GlobalText className="diatype-m-medium">#{height} Block</GlobalText>
          <TruncateText className="diatype-sm-regular text-tertiary-500" text={hash} end={20} />
        </View>
      </View>
    </MotiView>
  );
};

type SearchTransactionItemProps = { height: number; hash: string };

const TransactionItem: React.FC<SearchTransactionItemProps> = ({ height, hash }) => {
  return (
    <MotiView key={height} className="w-full p-2 min-h-[74px] rounded-xs">
      <View className="flex-row items-center gap-4">
        <View className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <EmojiTxs width={48} height={48} />
        </View>
        <View className="flex-col">
          <TruncateText className="flex-row gap-2 diatype-m-medium" text={hash} end={20} />
          <GlobalText className="diatype-sm-regular text-tertiary-500">Block: #{height}</GlobalText>
        </View>
      </View>
    </MotiView>
  );
};

type SearchContractItemProps = { contract: ContractInfo & { address: Address } };

const ContractItem: React.FC<SearchContractItemProps> = ({ contract }) => {
  const { address } = contract;

  return (
    <MotiView key={address} className="w-full p-2 min-h-[74px] rounded-xs">
      <View className="flex-row items-center gap-4">
        <View className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <EmojiFactory width={48} height={48} />
        </View>
        <View className="flex-col">
          <AddressVisualizer address={address} withIcon classNames={{ text: "diatype-m-medium" }} />
          <TruncateText className="diatype-sm-regular text-tertiary-500" text={address} end={20} />
        </View>
      </View>
    </MotiView>
  );
};

type SearchAccountItemProps = { account: Account };

const AccountItem: React.FC<SearchAccountItemProps> = ({ account }) => {
  const { username, address, type, index } = account;
  const name = `${username} - ${type} #${index}`;

  return (
    <MotiView key={address} className="w-full p-2 min-h-[74px] rounded-xs">
      <View className="flex-row items-center gap-4">
        <View className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <EmojiFactory width={48} height={48} />
        </View>
        <View className="flex-col">
          <GlobalText>{name}</GlobalText>
          <TruncateText className="diatype-sm-regular text-tertiary-500" text={address} end={20} />
        </View>
      </View>
    </MotiView>
  );
};

const ExportComponent = Object.assign(Root, {
  Applet: AppletItem,
  Asset: AssetItem,
  Block: BlockItem,
  Transaction: TransactionItem,
  Account: AccountItem,
  Contract: ContractItem,
});

export { ExportComponent as SearchItem };
