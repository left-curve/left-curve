import { createLazyFileRoute } from "@tanstack/react-router";

import type { Address } from "@left-curve/dango/types";
import { ContractExplorer } from "~/components/explorer/ContractExplorer";

import { m } from "~/paraglide/messages";
import { MobileTitle } from "~/components/foundation/MobileTitle";

export const Route = createLazyFileRoute("/(app)/_app/contract/$address")({
  component: ContractExplorerApplet,
});

function ContractExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle title={m["explorer.contracts.title"]()} className="p-4 pb-0" />
      <ContractExplorer address={address as Address}>
        <ContractExplorer.NotFound />
        <ContractExplorer.Details />
        <ContractExplorer.Assets />
      </ContractExplorer>
    </div>
  );
}
