import { createLazyFileRoute } from "@tanstack/react-router";

import { AccordionItem, Badge, TextCopy } from "@left-curve/applets-kit";
import { useQuery } from "@tanstack/react-query";
import { HeaderExplorer } from "~/components/explorer/HeaderExplorer";
import { ContractCard } from "~/components/foundation/ContractCard";

export const Route = createLazyFileRoute("/(app)/_app/contract/$address")({
  component: ContractExplorerApplet,
});

function ContractExplorerApplet() {
  const { address } = Route.useParams();

  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="flex flex-col gap-6 lg:flex-row">
        <ContractCard address={address} balance="2.34" />
        <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden w-full">
          <h1 className="h4-bold">Contract Detail</h1>
          <div className="flex flex-col gap-2">
            <div className="flex gap-1 items-center">
              <p className="diatype-md-medium text-gray-500">Code hash:</p>
              <p className="break-all overflow-hidden underline">
                CASFDF3425346455756686797897978563dfgfrrt5342serwe52343242432
              </p>
              <TextCopy className="w-4 h-4 text-gray-500" copyText={""} />
            </div>
            <div className="flex gap-1 items-center">
              <p className="diatype-md-medium text-gray-500">Admin:</p>
              <p className="break-all overflow-hidden">larry•spot#12</p>
            </div>
            <div className="flex gap-1 items-center">
              <p className="diatype-md-medium text-gray-500">Balances:</p>
              <Badge color="green" size="m" text="$125,000 (150 Tokens)" />
            </div>
          </div>
        </div>
      </div>

      <div className="w-full shadow-card-shadow bg-rice-25 rounded-3xl p-4 flex flex-col gap-4">
        Table
      </div>
    </div>
  );
}
