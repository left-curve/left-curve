import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Header } from "~/components/Header";

export const Route = createFileRoute("/(app)/_app")({
  component: function Layout() {
    return (
      <div className="flex flex-col min-h-screen w-full h-full relative scrollbar-none items-center justify-center">
        <img
          src="/images/union.png"
          alt="bg-image"
          className="drag-none select-none h-[20vh] w-full fixed top-0 left-0 z-0"
        />

        <Header />
        <main className="flex flex-1 w-full z-[2]">
          <Outlet />
        </main>
      </div>
    );
  },
});
