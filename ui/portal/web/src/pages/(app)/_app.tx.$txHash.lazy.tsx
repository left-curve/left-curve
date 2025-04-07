import { createLazyFileRoute } from "@tanstack/react-router";
import { TransactionExplorer } from "~/components/explorer/TransactionExplorer";

export const Route = createLazyFileRoute("/(app)/_app/tx/$txHash")({
  component: TransactionExplorerApplet,
});

function TransactionExplorerApplet() {
  const { txHash } = Route.useParams();

  return (
    <TransactionExplorer txHash={txHash}>
      <TransactionExplorer.NotFound />
      <TransactionExplorer.Details />
      <TransactionExplorer.Messages />
    </TransactionExplorer>
  );
}
