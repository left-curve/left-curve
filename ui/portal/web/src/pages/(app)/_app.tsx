import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Header } from "~/components/Header";

export const Route = createFileRoute("/(app)/_app")({
  component: function Layout() {
    return (
      <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-y-auto overflow-x-hidden scrollbar-none">
        <img
          src="/images/union.png"
          alt="bg-image"
          className="drag-none select-none h-[20vh] w-full fixed bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0"
        />

        <Header />
        <div className="flex items-start justify-center w-full z-10 h-full relative">
          <Outlet />
        </div>
      </main>
    );
  },
});
