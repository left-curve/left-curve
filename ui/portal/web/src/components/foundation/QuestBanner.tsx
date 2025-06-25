import { Button, twMerge } from "@left-curve/applets-kit";
import { IconChecked, IconClose } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { useApp } from "~/hooks/useApp";

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
      <p className={twMerge({ "text-tertiary-500": !completed })}>{text}</p>
    </div>
  );
};

export const QuestBanner: React.FC = () => {
  const { account, isConnected } = useAccount();
  const { isQuestBannerVisible, setQuestBannerVisibility } = useApp();

  const { data: quests } = useQuery({
    queryKey: ["quests", account?.username],
    enabled: isConnected && isQuestBannerVisible,
    queryFn: () =>
      fetch(`https://devnet.dango.exchange/quests/check_username/${account?.username}`).then(
        (res) => res.json(),
      ),
    initialData: () => ({
      eth_address: null,
      tx_count: null,
      limit_orders: false,
      market_orders: false,
      trading_pairs: Number.MAX_SAFE_INTEGER,
      trading_volumes: Number.MAX_SAFE_INTEGER,
    }),
  });

  const isTxCountCompleted = quests.tx_count >= 10;
  const isLimitOrdersCompleted = quests.limit_orders;
  const isMarketOrdersCompleted = quests.market_orders;
  const isTradingPairsCompleted = quests.trading_pairs === 0;
  const isTradingVolumesCompleted = quests.trading_volumes === 0;

  const areQuestsCompleted =
    quests.eth_address &&
    isTxCountCompleted &&
    isLimitOrdersCompleted &&
    isMarketOrdersCompleted &&
    isTradingPairsCompleted &&
    isTradingVolumesCompleted;

  if (!isQuestBannerVisible) return null;

  return (
    <div className="z-10 w-full shadow-account-card p-4 bg-account-card-blue flex gap-4 flex-col lg:flex-row lg:items-center justify-between relative">
      <a
        className="exposure-l-italic min-w-fit"
        href="https://app.galxe.com/quest/dango/GCpYut1Qnq"
        target="_blank"
        rel="noreferrer"
      >
        {m["quests.galxeQuest.title"]()}
      </a>
      <div className="flex flex-col lg:flex-row w-full justify-between gap-2">
        <div className="flex flex-col lg:flex-row gap-3 px-0 lg:px-4 lg:gap-6">
          <Quest
            text={`${m["quests.galxeQuest.quest"]({ quest: 0 })} ${quests.eth_address ? `(${quests.eth_address})` : ""}`}
            completed={!!quests.eth_address}
          />
          <Quest text={m["quests.galxeQuest.quest"]({ quest: 1 })} completed={isTxCountCompleted} />
          <Quest
            text={m["quests.galxeQuest.quest"]({ quest: 2 })}
            completed={isLimitOrdersCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest"]({ quest: 3 })}
            completed={isMarketOrdersCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest"]({ quest: 4 })}
            completed={isTradingPairsCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest"]({ quest: 5 })}
            completed={isTradingVolumesCompleted}
          />
        </div>
        {areQuestsCompleted ? (
          <Button
            as="a"
            href="https://app.galxe.com/quest/dango/GCpYut1Qnq"
            target="_blank"
            rel="noreferrer"
          >
            Claim NFT
          </Button>
        ) : null}
        <IconClose
          className="absolute top-4 right-4 lg:static h-6 w-6 text-tertiary-500 cursor-pointer"
          onClick={() => setQuestBannerVisibility(false)}
        />
      </div>
    </div>
  );
};
