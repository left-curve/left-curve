import { m } from "~/paraglide/messages";
import { MobileTitle } from "@left-curve/applets-kit";
import { createLazyFileRoute } from "@tanstack/react-router";
import { TransactionExplorer } from "~/components/explorer/TransactionExplorer";

export const Route = createLazyFileRoute("/(app)/_app/tx/$txHash")({
  component: TransactionExplorerApplet,
});

function TransactionExplorerApplet() {
  const { txHash } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle
        action={() => history.go(-1)}
        title={m["explorer.txs.title"]()}
        className="p-4 pb-0"
      />
      <TransactionExplorer txHash={txHash}>
        <TransactionExplorer.NotFound />
        <TransactionExplorer.Details />
        <TransactionExplorer.Messages />
      </TransactionExplorer>
    </div>
  );
}
