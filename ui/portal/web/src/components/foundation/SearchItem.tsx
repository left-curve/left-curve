import { AddressVisualizer, useMediaQuery } from "@left-curve/applets-kit";
import { useFavApplets } from "~/hooks/useFavApplets";

import { IconEmptyStar, IconStar, TruncateText } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import type { AppletMetadata } from "@left-curve/applets-kit";
import type { Account, Address, ContractInfo } from "@left-curve/dango/types";
import type { AnyCoin, WithPrice } from "@left-curve/store/types";
import type { MouseEvent, PropsWithChildren } from "react";

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

const Root: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type SearchAppletItemProps = AppletMetadata;

const AppletItem: React.FC<SearchAppletItemProps> = (applet) => {
  const { id, title, description, img } = applet;
  const { favApplets, addFavApplet, removeFavApplet } = useFavApplets();
  const isFav = favApplets[id];

  const onClickStar = (e: MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
    if (isFav) removeFavApplet(applet);
    else addFavApplet(applet);
  };

  return (
    <motion.div
      className="w-full p-2 flex items-center justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={title}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <img src={img} alt={title} className="w-12 h-12" />
        </div>
        <div>
          <p className="diatype-lg-medium text-secondary-700">{title}</p>
          <p className="diatype-m-regular text-tertiary-500">{description}</p>
        </div>
      </div>
      <div onClick={onClickStar}>
        {isFav ? (
          <IconStar className="w-6 h-6 text-rice-500" />
        ) : (
          <IconEmptyStar className="w-6 h-6" />
        )}
      </div>
    </motion.div>
  );
};

type SearchAssetProps = WithPrice<AnyCoin>;

const AssetItem: React.FC<SearchAssetProps> = ({ logoURI, symbol, price }) => {
  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={symbol}
    >
      <div className="flex items-start gap-4">
        <img src={logoURI} alt={symbol} className="w-8 h-8" />
        <div className="flex flex-col gap-1">
          <p className="diatype-m-bold">{symbol}</p>
          <p className="diatype-m-regular text-tertiary-500">{symbol}</p>
          {/* <p className="diatype-m-regular text-tertiary-500">{link}</p> */}
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <p className="diatype-sm-bold">${price}</p>
      </div>
    </motion.div>
  );
};

type SearchBlockItemProps = {
  height: number;
  hash: string;
};

const BlockItem: React.FC<SearchBlockItemProps> = ({ height, hash }) => {
  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={height}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <img src="/images/emojis/simple/blocks.svg" alt="test" className="w-12 h-12" />
        </div>
        <div className="flex flex-col">
          <p className="diatype-m-medium">#{height} Block</p>
          <TruncateText className="diatype-sm-regular text-tertiary-500" text={hash} end={20} />
        </div>
      </div>
    </motion.div>
  );
};

type SearchTransactionItemProps = {
  height: number;
  hash: string;
};

const TransactionItem: React.FC<SearchTransactionItemProps> = ({ height, hash }) => {
  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={height}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <img src="/images/emojis/simple/txs.svg" alt="test" className="w-12 h-12" />
        </div>
        <div className="flex flex-col">
          <TruncateText className="flex gap-2 diatype-m-medium" text={hash} end={20} />

          <p className="diatype-sm-regular text-tertiary-500">Block: #{height}</p>
        </div>
      </div>
    </motion.div>
  );
};

type SearchContractItemProps = {
  contract: ContractInfo & { address: Address };
};

const ContractItem: React.FC<SearchContractItemProps> = ({ contract }) => {
  const { address } = contract;
  const { isMd } = useMediaQuery();
  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={address}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <img src="/images/emojis/simple/factory.svg" alt="test" className="w-12 h-12" />
        </div>
        <div className="flex flex-col">
          <AddressVisualizer address={address} withIcon classNames={{ text: "diatype-m-medium" }} />
          {isMd ? (
            <p className="diatype-sm-regular text-tertiary-500">{address}</p>
          ) : (
            <TruncateText
              className="diatype-sm-regular text-tertiary-500"
              text={address}
              end={20}
            />
          )}
        </div>
      </div>
    </motion.div>
  );
};

type SearchAccountItemProps = {
  account: Account;
};

const AccountItem: React.FC<SearchAccountItemProps> = ({ account }) => {
  const { isMd } = useMediaQuery();
  const { username, address, type } = account;

  const name = `${username} - ${type} #${account?.index}`;

  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-surface-tertiary-rice rounded-xs group-data-[selected=true]:bg-surface-tertiary-rice cursor-pointer"
      variants={childVariants}
      key={address}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red">
          <img src={`/images/emojis/simple/${type}.svg`} alt={type} className="w-12 h-12" />
        </div>
        <div className="flex flex-col">
          <p className="flex gap-2 diatype-m-medium">{name}</p>
          {isMd ? (
            <p className="diatype-sm-regular text-tertiary-500">{address}</p>
          ) : (
            <TruncateText
              className="diatype-sm-regular text-tertiary-500"
              text={address}
              end={20}
            />
          )}
        </div>
      </div>
    </motion.div>
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
