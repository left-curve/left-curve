import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/points")({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/(app)/_app/points"!</div>;
}
