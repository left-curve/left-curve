import { createLazyFileRoute } from "@tanstack/react-router";
import { AccountExplorer } from "~/components/explorer/AccountExplorer";

import type { Address } from "@left-curve/dango/types";
import { MobileTitle } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/account/$address")({
  component: AccountExplorerApplet,
});

function AccountExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle
        action={() => history.go(-1)}
        title={m["explorer.accounts.title"]()}
        className="p-4 pb-0"
      />
      <AccountExplorer address={address as Address}>
        <AccountExplorer.NotFound />
        <AccountExplorer.Details />
        <AccountExplorer.Assets />
      </AccountExplorer>
    </div>
  );
}
