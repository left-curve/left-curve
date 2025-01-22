import { isValidAddress } from "@left-curve/dango";
import { createRoute, notFound } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";
import { AccountRouter } from "~/components/AccountRouter";

import type { AccountTypes } from "@left-curve/dango/types";
import { AppRoute } from "~/AppRouter";

export const AccountsRoute = createRoute({
  getParentRoute: () => AppRoute,
  path: "/accounts",
  validateSearch: zodValidator(
    z.object({
      address: z.custom((address) => address && isValidAddress(address)),
    }),
  ),
  loaderDeps: ({ search }) => ({ ...search }),
  loader: async ({ context, deps }) => {
    const { address } = deps;
    const { client } = context;
    const account = await client?.getAccountInfo({ address });

    if (!account) throw notFound();

    const type = Object.keys(account.params).at(0) as AccountTypes;

    const username = ["margin", "spot"].includes(type)
      ? (account.params as { [key: string]: { owner: string } })[type].owner
      : "";

    return {
      account: { ...account, username, type, address },
    };
  },
  notFoundComponent: () => {
    return <div>Account not found</div>;
  },
  component: () => {
    const { account } = AccountsRoute.useLoaderData();

    return (
      <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
        <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full">
          <AccountRouter account={account} />
        </div>
      </div>
    );
  },
});
