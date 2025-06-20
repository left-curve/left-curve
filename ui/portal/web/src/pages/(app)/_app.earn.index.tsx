import { createFileRoute } from "@tanstack/react-router";
import { Earn } from "~/components/earn/Earn";

export const Route = createFileRoute("/(app)/_app/earn/")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div className="w-full md:max-w-[76rem] mx-auto flex flex-col pt-6 mb-16">
      <Earn.Header />
      <Earn.PoolsCards />
      <Earn.UserPoolsTable />
    </div>
  );
}
