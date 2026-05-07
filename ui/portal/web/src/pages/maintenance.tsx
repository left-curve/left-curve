import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";

import { Spinner } from "@left-curve/applets-kit";
import { randomBetween } from "@left-curve/dango/utils";
import { Maintenance } from "~/components/foundation/Maintenance";

export const Route = createFileRoute("/maintenance")({
  component: MaintenanceApplet,
});

function MaintenanceApplet() {
  const navigate = useNavigate();

  const { data: isChainRunning, isFetched } = useQuery({
    queryKey: ["maintenance_chain_status"],
    queryFn: async () => {
      try {
        const response = await fetch(window.dango.urls.upUrl);
        if (!response.ok) throw new Error("request failed");
        const { is_running } = await response.json();
        return !!is_running;
      } catch {
        return false;
      }
    },
    refetchInterval: () => randomBetween(10_000, 30_000),
  });

  useEffect(() => {
    if (isChainRunning) navigate({ to: "/" });
  }, [isChainRunning]);

  if (!isFetched) {
    return (
      <div className="flex-1 w-full flex justify-center items-center h-screen">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <Maintenance />;
}
