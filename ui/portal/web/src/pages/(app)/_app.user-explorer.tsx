import { Badge } from "@left-curve/applets-kit";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/user-explorer")({
  component: UserExplorer,
});

function UserExplorer() {
  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="grid grid-cols-1 lg:grid-cols-[328px_1fr] gap-4">
        <div className="shadow-card-shadow bg-rice-25 rounded-xl p-4 flex gap-4 ">
          <div className="rounded-xs shadow-card-shadow bg-red-bean-50 p-[10px] w-[72px] h-[72px]">
            <img src="/images/emojis/simple/username.svg" alt="username-avatar" />
          </div>
          <div className="flex flex-col gap-4">
            <p className="h4-heavy text-gray-900">javier.user</p>
            <Badge
              text="Left Curve Trader"
              color="rice"
              className="bg-[linear-gradient(98.89deg,_#FFF5E6_18.66%,_#FCE5C4_46.73%,_#FFF5E6_86%)] rounded-md px-2 shadow-card-shadow"
            />
          </div>
        </div>
        <div className="shadow-card-shadow bg-rice-25 rounded-xl p-4 flex gap-4 flex-col">
          <div className="flex gap-2 items-center">
            <p className="h3-bold text-gray-900">$4,016</p>
            <p className="diatype-m-bold text-status-success">0.05% ($209.00)</p>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="flex flex-col gap-1">
              <p className="diatype-m-medium text-gray-500">Total Debt</p>
              <p className="diatype-m-bold text-gray-900">$100</p>
            </div>
            <div className="flex flex-col gap-1">
              <p className="diatype-m-medium text-gray-500">Total Assets</p>
              <p className="diatype-m-bold text-gray-900">$100.00</p>
            </div>
            <div className="flex flex-col gap-1">
              <p className="diatype-m-medium text-gray-500">Total Accounts</p>
              <p className="diatype-m-bold text-gray-900">12</p>
            </div>
            <div className="flex flex-col gap-1">
              <p className="diatype-m-medium text-gray-500">Date Joined</p>
              <p className="diatype-m-bold text-gray-900">10/09/2024, 12:08:03</p>
            </div>
          </div>
        </div>
      </div>
      <div className="p-4  bg-rice-50 rounded-xl shadow-card-shadow" />
    </div>
  );
}
