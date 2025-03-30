import { twMerge } from "@left-curve/applets-kit";
import { IconChecked, IconClose } from "@left-curve/applets-kit";
import { useAccount, useStorage } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { m } from "~/paraglide/messages";

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

export const QuestBanner: React.FC = () => {
  const { account, isConnected } = useAccount();
  const [showGalxeQuestBanner, setShowGalxeQuestBanner] = useStorage("showGalxeQuestBanner", {
    initialValue: true,
  });

  const { data: quests } = useQuery({
    queryKey: ["quests", account],
    enabled: isConnected,
    queryFn: () =>
      fetch(`https://devnet.dango.exchange/quests/check_username/${account?.username}`).then(
        (res) => res.json(),
      ),
    initialData: () => ({
      eth_address: null,
      quest_account: false,
      quest_transfer: false,
    }),
  });

  const { eth_address, quest_account, quest_transfer } = quests;

  if (!showGalxeQuestBanner) return null;

  return (
    <div className="max-w-[76rem] mx-auto pt-4 px-4 z-10 w-full">
      <div className="w-full rounded-xl shadow-card-shadow p-4 bg-account-card-blue flex gap-4 flex-col lg:flex-row lg:items-center justify-between relative">
        <p className="exposure-l-italic min-w-fit">{m["quests.galxeQuest.title"]()}</p>
        <div className="flex w-full justify-between">
          <div className="flex flex-col lg:flex-row gap-3 px-0 lg:px-4 lg:gap-6">
            <Quest text={m["quests.galxeQuest.quest"]({ quest: 0 })} completed={!!eth_address} />
            <Quest text={m["quests.galxeQuest.quest"]({ quest: 1 })} completed={quest_account} />
            <Quest text={m["quests.galxeQuest.quest"]({ quest: 2 })} completed={quest_transfer} />
          </div>
          <IconClose
            className="absolute top-4 right-4 lg:static h-6 w-6 text-gray-400 cursor-pointer"
            onClick={() => setShowGalxeQuestBanner(false)}
          />
        </div>
      </div>
    </div>
  );
};
