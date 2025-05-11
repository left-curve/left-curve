import { createLazyFileRoute } from "@tanstack/react-router";

import type { Address } from "@left-curve/dango/types";
import { ContractExplorer } from "~/components/explorer/ContractExplorer";

export const Route = createLazyFileRoute("/(app)/_app/contract/$address")({
  component: ContractExplorerApplet,
});

function ContractExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <ContractExplorer address={address as Address}>
      <ContractExplorer.NotFound />
      <ContractExplorer.Details />
      <ContractExplorer.Assets />
    </ContractExplorer>
  );
}
