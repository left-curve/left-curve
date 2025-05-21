import { createLazyFileRoute } from "@tanstack/react-router";

import type { Address } from "@left-curve/dango/types";
import { ContractExplorer } from "~/components/explorer/ContractExplorer";
import { MobileTitle } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/contract/$address")({
  component: ContractExplorerApplet,
});

function ContractExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle
        action={() => history.go(-1)}
        title={m["explorer.contracts.title"]()}
        className="p-4 pb-0"
      />
      <ContractExplorer address={address as Address}>
        <ContractExplorer.NotFound />
        <ContractExplorer.Details />
        <ContractExplorer.Assets />
      </ContractExplorer>
    </div>
  );
}
