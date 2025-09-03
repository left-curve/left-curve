import { useStorage } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { useAccount, useAppConfig, useConfig, usePublicClient } from "@left-curve/store";

import DGXLogo from "@left-curve/foundation/images/pwa.png";
import TruncateText from "./TruncateText";
import { IconLink } from "./icons/IconLink";
import { View, Text, Pressable } from "react-native";
import { IconUserCircle } from "./icons/IconUserCircle";

import { twMerge } from "@left-curve/foundation";

import type React from "react";
import type { ViewProps } from "react-native";
import type { Address, AllLeafKeys, AppConfig } from "@left-curve/dango/types";

type AddressVisualizerProps = {
  address: Address;
  classNames?: {
    container?: string;
    text?: string;
  };
  withIcon?: boolean;
  onClick?: (url: string) => void;
} & ViewProps;

type AddressInfo = { name: string; type: string };

const DANGO_CONTRACT_NAMES: Record<AllLeafKeys<AppConfig["addresses"]>, string> = {
  accountFactory: "Account Factory",
  dex: "DEX",
  gateway: "Gateway",
  ism: "ISM",
  lending: "Lending",
  mailbox: "Mailbox",
  oracle: "Oracle",
  taxman: "Taxman",
  va: "VA",
  warp: "Warp",
};

export const AddressVisualizer: React.FC<AddressVisualizerProps> = ({
  address,
  classNames,
  withIcon,
  onClick,
  ...rest
}) => {
  const { data: config } = useAppConfig();
  const { chain } = useConfig();
  const { accounts } = useAccount();
  const client = usePublicClient();

  const [addresses, setAddresses] = useStorage<
    Record<string, { contract?: AddressInfo; account?: AddressInfo }>
  >("app.known_addresses", {
    initialValue: {},
    sync: true,
  });

  const blockExplorer = chain.blockExplorer;
  const isClickable = !!onClick;

  const { data } = useQuery({
    queryKey: ["address_visualizer", config, address],
    queryFn: async () => {
      if (addresses[address]) return addresses[address];

      const contractKey = (config?.addresses as any)?.[address] as
        | AllLeafKeys<AppConfig["addresses"]>
        | undefined;

      if (contractKey) {
        const info = {
          contract: {
            name: DANGO_CONTRACT_NAMES[contractKey],
            type: "dango",
          } as AddressInfo,
        };
        setAddresses((prev) => ({ ...prev, [address]: info }));
        return info;
      }

      const userAccount = accounts?.find((a) => a.address === address);
      if (userAccount) {
        const info = {
          account: {
            name: `${userAccount.username} #${userAccount.index}`,
            type: "own",
          } as AddressInfo,
        };
        setAddresses((prev) => ({ ...prev, [address]: info }));
        return info;
      }

      const acc = await client.getAccountInfo({ address });
      if (acc) {
        const info = {
          account: {
            name: `${acc.username} #${acc.index}`,
            type: "other",
          } as AddressInfo,
        };
        setAddresses((prev) => ({ ...prev, [address]: info }));
        return info;
      }

      const contract = await client.getContractInfo({ address });
      if (contract?.label) {
        const info = {
          contract: {
            name: contract.label,
            type: "other",
          } as AddressInfo,
        };
        setAddresses((prev) => ({ ...prev, [address]: info }));
        return info;
      }

      return {};
    },
  });

  const { contract, account } = (data || {}) as { contract?: AddressInfo; account?: AddressInfo };

  const Container = isClickable ? Pressable : View;

  const contractUrl = blockExplorer.contractPage.replace("${address}", address);
  const accountUrl = blockExplorer.accountPage.replace("${address}", address);

  if (contract) {
    return (
      <Container
        onPress={isClickable ? () => onClick?.(contractUrl) : undefined}
        accessibilityRole={isClickable ? "button" : undefined}
        className={twMerge("flex-row items-center gap-1", classNames?.container)}
        {...rest}
      >
        {withIcon ? <DGXLogo width={16} height={16} /> : null}
        <Text className={twMerge("diatype-m-bold", classNames?.text)}>{contract.name}</Text>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </Container>
    );
  }

  if (account) {
    return (
      <Container
        onPress={isClickable ? () => onClick?.(accountUrl) : undefined}
        accessibilityRole={isClickable ? "button" : undefined}
        className={twMerge("flex-row items-center gap-1", classNames?.container)}
        {...rest}
      >
        {withIcon ? (
          <IconUserCircle className="w-4 h-4 fill-rice-50 text-rice-500 rounded-full overflow-hidden" />
        ) : null}
        <Text className={twMerge("diatype-m-bold", classNames?.text)}>{account.name}</Text>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </Container>
    );
  }

  return <TruncateText text={address} className={classNames?.text} />;
};
