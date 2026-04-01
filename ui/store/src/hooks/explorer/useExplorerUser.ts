import { useQueries, useQuery } from "@tanstack/react-query";
import { usePublicClient } from "../usePublicClient.js";
import { usePrices } from "../usePrices.js";

import type { Address, Coins, User, UserStatus } from "@left-curve/dango/types";

export type AccountWithDetails = {
  address: Address;
  index: number;
  balance: Coins;
  balanceUSD: string;
  isActive: boolean;
};

export type ExplorerUserData = {
  user: User;
  accounts: AccountWithDetails[];
  aggregatedBalances: Coins;
  totalValue: string;
  totalAccounts: number;
};

export function useExplorerUser(username: string) {
  const client = usePublicClient();
  const { calculateBalance } = usePrices();

  const userQuery = useQuery<User | null>({
    queryKey: ["explorer_user", username],
    queryFn: async () => {
      try {
        return await client.getUser({ userIndexOrName: { name: username } });
      } catch {
        return null;
      }
    },
    enabled: !!username,
  });

  const accountAddresses = userQuery.data
    ? Object.values(userQuery.data.accounts)
    : [];

  const accountQueries = useQueries({
    queries: accountAddresses.map((address, idx) => ({
      queryKey: ["explorer_user_account", address],
      queryFn: async () => {
        const [balances, status] = await Promise.all([
          client.getBalances({ address }),
          client.getAccountStatus({ address }).catch(() => "inactive" as UserStatus),
        ]);

        const balanceUSD = calculateBalance(balances, { format: true });

        return {
          address,
          index: Number(Object.keys(userQuery.data!.accounts)[idx]),
          balance: balances,
          balanceUSD,
          isActive: status === "active",
        } satisfies AccountWithDetails;
      },
      enabled: !!userQuery.data && accountAddresses.length > 0,
    })),
  });

  const isAccountsLoading = accountQueries.some((q) => q.isLoading);
  const accounts = accountQueries
    .filter((q) => q.data)
    .map((q) => q.data as AccountWithDetails);

  const aggregatedBalances: Coins = {};
  for (const account of accounts) {
    for (const [denom, amount] of Object.entries(account.balance)) {
      const current = BigInt(aggregatedBalances[denom] || "0");
      const toAdd = BigInt(amount);
      aggregatedBalances[denom] = (current + toAdd).toString();
    }
  }

  const totalValue = calculateBalance(aggregatedBalances, { format: true });

  return {
    data: userQuery.data
      ? ({
          user: userQuery.data,
          accounts,
          aggregatedBalances,
          totalValue,
          totalAccounts: accountAddresses.length,
        } satisfies ExplorerUserData)
      : null,
    isLoading: userQuery.isLoading || isAccountsLoading,
    isNotFound: !userQuery.isLoading && !userQuery.data,
    error: userQuery.error,
  };
}
