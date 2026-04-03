import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { VaultLiquidity } from "~/components/earn/VaultLiquidity";

export const Route = createLazyFileRoute("/(app)/_app/earn/vault")({
  component: VaultApplet,
});

function VaultApplet() {
  const { action } = Route.useSearch();
  const navigate = useNavigate();

  const onChangeAction = (action: "deposit" | "withdraw") => {
    navigate({
      to: "/earn/vault",
      replace: true,
      search: { action },
    });
  };

  return (
    <VaultLiquidity action={action} onChangeAction={onChangeAction}>
      <VaultLiquidity.Header />
      <div className="flex w-full gap-8 lg:gap-6 flex-col lg:flex-row">
        <div className="flex flex-col flex-1 min-w-0 gap-4">
          <VaultLiquidity.HeaderTabs />
          <VaultLiquidity.Deposit />
          <VaultLiquidity.Withdraw />
        </div>
        <VaultLiquidity.UserLiquidity />
      </div>
    </VaultLiquidity>
  );
}
