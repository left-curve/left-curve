import { createLazyFileRoute } from "@tanstack/react-router";
import { Transaction } from "~/components/explorer/Transaction";

export const Route = createLazyFileRoute("/(app)/_app/tx/$txHash")({
  component: TransactionExplorer,
});

function TransactionExplorer() {
  const { txHash } = Route.useParams();

  return (
    <Transaction txHash={txHash}>
      <Transaction.NotFound />
      <Transaction.Details />
      <Transaction.Messages />
    </Transaction>
  );
}
