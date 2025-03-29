import { twMerge } from "@left-curve/applets-kit";
import { IconChecked, IconClose } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

const Quest: React.FC<{ text: string; completed: boolean }> = ({ completed, text }) => {
  return (
    <div className="flex items-center gap-1 diatype-sm-medium">
      <div
        className={twMerge(
          "h-4 w-4 flex items-center justify-center rounded-full bg-green-bean-400",
          { "bg-gray-400": !completed },
        )}
      >
        {completed ? (
          <IconChecked className="h-3 w-3 text-white" />
        ) : (
          <IconClose className="h-4 w-4 text-white" />
        )}
      </div>
      <p className={twMerge({ "text-gray-400": !completed })}>{text}</p>
    </div>
  );
};

export const Quests: React.FC = () => {
  const { account, isConnected } = useAccount();

  const { data: quests, isLoading } = useQuery({
    queryKey: ["quests", account],
    enabled: isConnected,
    queryFn: () => fetch("/api/quests").then((res) => res.json()),
    initialData: () => ({
      eth_address: "",
      quest_account: false,
      quest_transfer: false,
    }),
  });

  const { eth_address, quest_account, quest_transfer } = quests;

  return (
    <div className="w-full rounded-lg shadow-card-shadow p-4 bg-account-card-blue flex gap-4 flex-col lg:flex-row lg:items-center justify-between">
      <p className="exposure-l-italic">Quests</p>
      <div className="flex lg:items-center gap-3 lg:gap-6 flex-col lg:flex-row">
        <Quest text="Add ethereum key" completed={eth_address.length} />
        <Quest text="Create a sub account" completed={quest_account} />
        <Quest text="Make your first transfer" completed={quest_transfer} />
      </div>
    </div>
  );
};
