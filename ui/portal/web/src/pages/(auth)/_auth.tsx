import { Outlet, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(auth)/_auth")({
  component: function Layout() {
    return <Outlet />;
  },
});
