import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/maintenance")({
  component: MaintenanceApplet,
});

import { Maintenance } from "~/components/foundation/Maintenance";

function MaintenanceApplet() {
  return <Maintenance />;
}
