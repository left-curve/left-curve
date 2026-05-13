/** biome-ignore-all lint/suspicious/noTemplateCurlyInString: it uses template literals for URL replacement */

import { useAccount, useAppConfig, useConfig, usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { twMerge } from "@left-curve/foundation";

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
  perps: "Perps",
};

export const AddressVisualizer: React.FC<AddressVisualizerProps> = ({
  address,
  classNames,
  withIcon,
  onClick,
}) => {
  const { data: config } = useAppConfig();
  const { chain } = useConfig();
  const { accounts, username: currentUsername } = useAccount();
  const client = usePublicClient();

  const blockExplorer = chain.blockExplorer;

  const isClickable = !!onClick;

  const { data } = useQuery({
    queryKey: ["address_visualizer", config, address],
    queryFn: async () => {
      const contractName = config.addresses[address as keyof typeof config.addresses] as string;
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
            name: currentUsername
              ? `${currentUsername} #${userAccount.index}`
              : `Account #${userAccount.index}`,
            type: "own",
          },
        };
      }

      const account = await client.getAccountInfo({ address });

      if (account) {
        return {
          account: {
            name: `${account.username} #${account.index}`,
            type: "other",
          },
        };
      }

      const contract = await client.getContractInfo({ address });

      if (contract?.label) {
        return {
          contract: {
            name: contract.label,
            type: "other",
          },
        };
      }

      return {};
    },
  });

  const { contract, account } = (data || {}) as { contract?: AddressInfo; account?: AddressInfo };

  const contractUrl = blockExplorer.contractPage.replace("${address}", address);
  const accountUrl = blockExplorer.accountPage.replace("${address}", address);

  if (contract) {
    const content = (
      <>
        {withIcon ? <img src="/DGX.svg" alt="dango logo" className="h-4 w-4" /> : null}
        <span className={twMerge("diatype-m-bold", classNames?.text)}>{contract.name}</span>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </>
    );

    return isClickable ? (
      <a
        href={contractUrl}
        className={twMerge("flex items-center gap-1 cursor-pointer", classNames?.container)}
        onClick={(e) => {
          e.preventDefault();
          onClick?.(contractUrl);
        }}
      >
        {content}
      </a>
    ) : (
      <div className={twMerge("flex items-center gap-1", classNames?.container)}>{content}</div>
    );
  }

  if (account) {
    const content = (
      <>
        {withIcon ? (
          <IconUserCircle className="w-4 h-4 fill-primitives-rice-light-50 text-primitives-rice-light-500 rounded-full overflow-hidden" />
        ) : null}
        <span className={twMerge("diatype-m-bold", classNames?.text)}>{account.name}</span>
        {isClickable ? <IconLink className="w-4 h-4" /> : null}
      </>
    );

    return isClickable ? (
      <a
        href={accountUrl}
        className={twMerge("flex items-center gap-1 cursor-pointer", classNames?.container)}
        onClick={(e) => {
          e.preventDefault();
          onClick?.(accountUrl);
        }}
      >
        {content}
      </a>
    ) : (
      <div className={twMerge("flex items-center gap-1", classNames?.container)}>{content}</div>
    );
  }

  return <TruncateResponsive text={address} className={classNames?.text} />;
};
