import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppLayout } from "~/components/AppLayout";

export const Route = createFileRoute("/(app)/_app")({
  component: function Layout() {
    return (
      <AppLayout>
        <Outlet />
      </AppLayout>
    );
  },
});
