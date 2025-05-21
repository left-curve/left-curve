import { createLazyFileRoute } from "@tanstack/react-router";
import { AccountExplorer } from "~/components/explorer/AccountExplorer";

import type { Address } from "@left-curve/dango/types";

import { m } from "~/paraglide/messages";
import { MobileTitle } from "~/components/foundation/MobileTitle";

export const Route = createLazyFileRoute("/(app)/_app/account/$address")({
  component: AccountExplorerApplet,
});

function AccountExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle title={m["explorer.accounts.title"]()} className="p-4 pb-0" />
      <AccountExplorer address={address as Address}>
        <AccountExplorer.NotFound />
        <AccountExplorer.Details />
        <AccountExplorer.Assets />
      </AccountExplorer>
    </div>
  );
}
