import { m } from "~/paraglide/messages";
import { createLazyFileRoute } from "@tanstack/react-router";

import { MobileTitle } from "@left-curve/applets-kit";
import { BlockExplorer } from "~/components/explorer/BlockExplorer";

export const Route = createLazyFileRoute("/(app)/_app/block/$block")({
  component: BlockExplorerApplet,
});

function BlockExplorerApplet() {
  const { block } = Route.useParams();

  return (
    <div className="w-full flex flex-col">
      <MobileTitle
        action={() => history.go(-1)}
        title={m["explorer.block.title"]()}
        className="p-4 pb-0"
      />
      <BlockExplorer height={block}>
        <BlockExplorer.Skeleton />
        <BlockExplorer.NotFound />
        <BlockExplorer.FutureBlock />
        <BlockExplorer.Details />
        <BlockExplorer.TxTable />
      </BlockExplorer>
    </div>
  );
}
