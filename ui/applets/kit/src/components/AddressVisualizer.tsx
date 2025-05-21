import { useAccount, useAppConfig, usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { useMediaQuery } from "#hooks/useMediaQuery.js";

import { twMerge } from "#utils/twMerge.js";
import TruncateText from "./TruncateText";
import { IconUserCircle } from "./icons/IconUserCircle";

import type { Address } from "@left-curve/dango/types";
import { camelToTitleCase } from "@left-curve/dango/utils";
import type React from "react";

type AddressVisualizerProps = {
  address: Address;
  className?: string;
  truncate?: boolean;
  withIcon?: boolean;
};

export const AddressVisualizer: React.FC<AddressVisualizerProps> = ({
  address,
  truncate,
  className,
  withIcon,
}) => {
  const { isMd } = useMediaQuery();
  const { data: config } = useAppConfig();
  const { accounts } = useAccount();
  const client = usePublicClient();

  const { data: account } = useQuery({
    queryKey: ["address_visualizer", address],
    queryFn: () => client.getAccountInfo({ address }),
  });

  const dangoContract = config?.addresses[address as keyof typeof config.addresses] as string;

  if (dangoContract)
    return (
      <p className={twMerge("flex items-center gap-1", className)}>
        {withIcon ? <img src="/favicon.svg" alt="dango logo" className="h-4 w-4" /> : null}
        <span className="diatype-m-bold">{camelToTitleCase(dangoContract)}</span>
      </p>
    );

  const userAccount = accounts?.find((account) => account.address === address);

  const anyAccount = userAccount || account;

  if (anyAccount)
    return (
      <p className={twMerge("flex items-center gap-1", className)}>
        {withIcon ? <IconUserCircle className="w-4 h-4 fill-rice-50 text-rice-500" /> : null}
        <span className="diatype-m-bold">{`${anyAccount.username} #${anyAccount.index}`}</span>
      </p>
    );

  return !isMd && truncate ? (
    <TruncateText className={className} text={address} />
  ) : (
    <p className={className}>{address}</p>
  );
};
