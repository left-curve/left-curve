import { createFileRoute } from "@tanstack/react-router";
import { EarnHeader } from "~/components/earn/EarnHeader";
import { PoolsTable } from "~/components/earn/PoolsTable";
import { StrategySection } from "~/components/earn/StrategySection";

export const Route = createFileRoute("/(app)/_app/earn")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div className="w-full md:max-w-[76rem] mx-auto flex flex-col pt-6 mb-16">
      <EarnHeader />
      <StrategySection />
      <PoolsTable />
    </div>
  );
}
