import { m } from "~/paraglide/messages";

import { createLazyFileRoute } from "@tanstack/react-router";
import { TransactionExplorer } from "~/components/explorer/TransactionExplorer";
import { MobileTitle } from "~/components/foundation/MobileTitle";

export const Route = createLazyFileRoute("/(app)/_app/tx/$txHash")({
  component: TransactionExplorerApplet,
});

function TransactionExplorerApplet() {
  const { txHash } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle title={m["explorer.txs.title"]()} className="p-4 pb-0" />
      <TransactionExplorer txHash={txHash}>
        <TransactionExplorer.NotFound />
        <TransactionExplorer.Details />
        <TransactionExplorer.Messages />
      </TransactionExplorer>
    </div>
  );
}
