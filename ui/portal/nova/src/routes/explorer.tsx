import { createRoute, Outlet, useNavigate } from "@tanstack/react-router";
import { View } from "react-native";
import { Route as rootRoute } from "./__root";
import { ExplorerScreen } from "../explorer/ExplorerScreen";
import { BlockDetail } from "../explorer/BlockDetail";
import { TxDetail } from "../explorer/TxDetail";

function ExplorerLayout() {
  return (
    <View className="flex-1">
      <Outlet />
    </View>
  );
}

export const explorerRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/explorer",
  component: ExplorerLayout,
});

export const explorerIndexRoute = createRoute({
  getParentRoute: () => explorerRoute,
  path: "/",
  component: ExplorerScreen,
});

export const explorerBlockRoute = createRoute({
  getParentRoute: () => explorerRoute,
  path: "/block/$block",
  component: function ExplorerBlockDetail() {
    const { block } = explorerBlockRoute.useParams();
    const navigate = useNavigate();
    return (
      <BlockDetail
        blockHeight={Number(block)}
        onTxPress={(hash) => navigate({ to: `/explorer/tx/${hash}` })}
        onBack={() => window.history.back()}
      />
    );
  },
});

export const explorerTxRoute = createRoute({
  getParentRoute: () => explorerRoute,
  path: "/tx/$txHash",
  component: function ExplorerTxDetail() {
    const { txHash } = explorerTxRoute.useParams();
    const navigate = useNavigate();
    return (
      <TxDetail
        txHash={txHash}
        onBlockPress={(height) => navigate({ to: `/explorer/block/${height}` })}
        onBack={() => window.history.back()}
      />
    );
  },
});
