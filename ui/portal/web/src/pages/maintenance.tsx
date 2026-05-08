import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";

import { Spinner } from "@left-curve/applets-kit";
import { Maintenance } from "~/components/foundation/Maintenance";

export const Route = createFileRoute("/maintenance")({
  component: MaintenanceApplet,
});

const RECOVERY_THRESHOLD = 3;

function MaintenanceApplet() {
  const navigate = useNavigate();
  const successCount = useRef(0);

  const { data: isChainRunning, isFetched } = useQuery({
    queryKey: ["maintenance_chain_status"],
    queryFn: async () => {
      try {
        const response = await fetch(window.dango.urls.upUrl);
        if (!response.ok) throw new Error("request failed");
        const { is_running } = await response.json();
        if (!is_running) {
          successCount.current = 0;
          return false;
        }
        successCount.current += 1;
        return successCount.current >= RECOVERY_THRESHOLD;
      } catch {
        successCount.current = 0;
        return false;
      }
    },
    refetchInterval: 10_000,
  });

  useEffect(() => {
    if (isChainRunning && window.location.pathname === "/maintenance") {
      navigate({ to: "/" });
    }
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
