import { useAccount, useAppConfig, useConfig, usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { twMerge } from "#utils/twMerge.js";
import { camelToTitleCase } from "@left-curve/dango/utils";

import { IconUserCircle } from "./icons/IconUserCircle";
import { TruncateResponsive } from "./TruncateResponsive";

import type React from "react";
import type { Address } from "@left-curve/dango/types";
import { IconLink } from "./icons/IconLink";

type AddressVisualizerProps = {
  address: Address;
  className?: string;
  withIcon?: boolean;
  onClick?: (url: string) => void;
};

export const AddressVisualizer: React.FC<AddressVisualizerProps> = ({
  address,
  className,
  withIcon,
  onClick,
}) => {
  const { data: config } = useAppConfig();
  const { chain } = useConfig();
  const { accounts } = useAccount();
  const client = usePublicClient();

  const blockExplorer = chain.blockExplorer;

  const isOnClickAvailable = !!onClick;

  const { data: account } = useQuery({
    queryKey: ["address_visualizer", address],
    queryFn: () => client.getAccountInfo({ address }),
  });

  const dangoContract = config?.addresses[address as keyof typeof config.addresses] as string;

  if (dangoContract)
    return (
      <p
        className={twMerge(
          "flex items-center gap-1 diatype-m-bold",
          { "cursor-pointer": isOnClickAvailable },
          className,
        )}
        onClick={() => onClick?.(blockExplorer.contractPage.replace("${address}", address))}
      >
        {withIcon ? <img src="/DGX.svg" alt="dango logo" className="h-4 w-4" /> : null}
        <span>{camelToTitleCase(dangoContract).replace("dex", "DEX")}</span>
        {isOnClickAvailable ? <IconLink className="w-4 h-4" /> : null}
      </p>
    );

  const userAccount = accounts?.find((account) => account.address === address);

  const anyAccount = userAccount || account;

  if (anyAccount)
    return (
      <p
        className={twMerge(
          "flex items-center gap-1",
          { "cursor-pointer": isOnClickAvailable },
          className,
        )}
        onClick={() =>
          onClick?.(blockExplorer.accountPage.replace("${address}", anyAccount.address))
        }
      >
        {withIcon ? (
          <IconUserCircle className="w-4 h-4 fill-rice-50 text-rice-500 rounded-full overflow-hidden" />
        ) : null}
        <span className="diatype-m-bold">{`${anyAccount.username} #${anyAccount.index}`}</span>
        {isOnClickAvailable ? <IconLink className="w-4 h-4" /> : null}
      </p>
    );

  return <TruncateResponsive text={address} className={className} />;
};
