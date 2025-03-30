import { createLazyFileRoute } from "@tanstack/react-router";
import { BlockExplorer } from "~/components/explorer/BlockExplorer";

export const Route = createLazyFileRoute("/(app)/_app/block/$block")({
  component: BlockExplorerApplet,
});

function BlockExplorerApplet() {
  const { block } = Route.useParams();

  return (
    <BlockExplorer height={block}>
      <BlockExplorer.Skeleton />
      <BlockExplorer.NotFound />
      <BlockExplorer.FutureBlock />
      <BlockExplorer.Details />
      <BlockExplorer.TxTable />
    </BlockExplorer>
  );
}
