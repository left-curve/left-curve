import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/pro-swap")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div className="flex p-4">Pro swap route</div>;
}
