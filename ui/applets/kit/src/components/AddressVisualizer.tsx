/** biome-ignore-all lint/suspicious/noTemplateCurlyInString: it uses template literals for URL replacement */

import { useAccount, useAppConfig, useConfig, usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { twMerge } from "@left-curve/foundation";
import { useStorage } from "@left-curve/store";

import { TruncateResponsive } from "./TruncateResponsive";
import { IconLink } from "./icons/IconLink";
import { IconUserCircle } from "./icons/IconUserCircle";

import type { Address, AllLeafKeys, AppConfig } from "@left-curve/dango/types";
import type React from "react";

type AddressVisualizerProps = {
  address: Address;
  classNames?: {
    container?: string;
    text?: string;
  };
  withIcon?: boolean;
  onClick?: (url: string) => void;
};

type AddressInfo = {
  name: string;
  type: string;
};

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
}) => {
  const { data: config } = useAppConfig();
  const { chain } = useConfig();
  const { accounts } = useAccount();
  const client = usePublicClient();

  const [addresses, setAddresses] = useStorage<
    Record<string, { contract: AddressInfo } | { account: AddressInfo }>
  >("app.known_addresses", {
    initialValue: {},
    sync: true,
  });

  const blockExplorer = chain.blockExplorer;

  const isClickable = !!onClick;

  const Component = isClickable ? "button" : "div";

  const { data } = useQuery({
    queryKey: ["address_visualizer", config, address],
    queryFn: async () => {
      if (addresses[address]) return addresses[address];

      const contractName = config?.addresses[address as keyof typeof config.addresses] as string;
      if (contractName) {
        return {
          contract: {
            name: DANGO_CONTRACT_NAMES[contractName as keyof typeof DANGO_CONTRACT_NAMES],
            type: "dango",
          },
        };
      }

      const userAccount = accounts?.find((account) => account.address === address);

      if (userAccount) {
        return {
          account: {
            name: `${userAccount.username} #${userAccount.index}`,
            type: "own",
          },
        };
      }

      const account = await client.getAccountInfo({ address });

      if (account) {
        const accountName = `${account.username} #${account.index}`;
        const type = "other";
        const info = { account: { name: accountName, type } };

        setAddresses((prev) => ({ ...prev, [address]: info }));
        return info;
      }

      const contract = await client.getContractInfo({ address });

      if (contract?.label) {
        const contractName = contract.label;
        const type = "other";
        const info = { contract: { name: contractName, type } };

        setAddresses((prev) => ({ ...prev, [address]: info }));
        return { contract: contractName };
      }

      return {};
    },
  });

  const { contract, account } = (data || {}) as { contract?: AddressInfo; account?: AddressInfo };

  if (contract)
    return (
      <Component
        className={twMerge(
          "flex items-center gap-1",
          { "cursor-pointer": isClickable },
          classNames?.container,
        )}
        onClick={() => onClick?.(blockExplorer.contractPage.replace("${address}", address))}
      >
        {withIcon ? <img src="/DGX.svg" alt="dango logo" className="h-4 w-4" /> : null}
        <span className={twMerge("diatype-m-bold", classNames?.text)}>{contract.name}</span>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </Component>
    );

  if (account)
    return (
      <Component
        className={twMerge(
          "flex items-center gap-1",
          { "cursor-pointer": isClickable },
          classNames?.container,
        )}
        onClick={() => onClick?.(blockExplorer.accountPage.replace("${address}", address))}
      >
        {withIcon ? (
          <IconUserCircle className="w-4 h-4 fill-rice-50 text-rice-500 rounded-full overflow-hidden" />
        ) : null}
        <span className={twMerge("diatype-m-bold", classNames?.text)}>{account.name}</span>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </Component>
    );

  return <TruncateResponsive text={address} className={classNames?.text} />;
};
