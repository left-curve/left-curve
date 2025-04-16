import { createLazyFileRoute } from "@tanstack/react-router";
import { AccountExplorer } from "~/components/explorer/AccountExplorer";

import type { Address } from "@left-curve/dango/types";

export const Route = createLazyFileRoute("/(app)/_app/account/$address")({
  component: AccountExplorerApplet,
});

function AccountExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <AccountExplorer address={address as Address}>
      <AccountExplorer.NotFound />
      <AccountExplorer.Details />
      <AccountExplorer.Assets />
    </AccountExplorer>
  );
}
