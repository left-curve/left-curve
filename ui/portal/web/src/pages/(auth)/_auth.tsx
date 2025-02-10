import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppLayout } from "~/components/AppLayout";

export const Route = createFileRoute("/(auth)/_auth")({
  component: function Layout() {
    return (
      <AppLayout>
        <Outlet />
      </AppLayout>
    );
  },
});
