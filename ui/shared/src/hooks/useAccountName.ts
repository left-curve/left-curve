import { useAccount, useStorage } from "@leftcurve/react";
import type { Account } from "@leftcurve/types";

export type UseAccountNameParameters = {
  account?: Account;
};

export type UseAccountNameReturnType = [string, (name: string) => void];

export function useAccountName(
  parameters: UseAccountNameParameters = {},
): UseAccountNameReturnType {
  const { account: acc } = useAccount();
  const [names, setNames] = useStorage<Record<string, string>>("account_names", {
    initialValue: {},
  });

  const account = parameters.account || acc;

  const name = !account ? "" : names[account?.address] || `${account.type} #${account.index}`;
  const setName = (name: string) => {
    if (!account) return;
    setNames({ ...names, [account.address]: name });
  };
  return [name, setName];
}
