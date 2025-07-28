import { Button, twMerge } from "@left-curve/applets-kit";
import { IconChecked, IconClose } from "@left-curve/applets-kit";
import { Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { useAccount } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";
import { QUEST_URI } from "~/store";

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
  const { isQuestBannerVisible, setQuestBannerVisibility, settings } = useApp();
  const { formatNumberOptions } = settings;

  const { data: quests, isLoading } = useQuery({
    queryKey: ["quests", account?.username],
    enabled: isConnected && isQuestBannerVisible,
    queryFn: () => fetch(`${QUEST_URI}/${account?.username}`).then((res) => res.json()),
  });

  const isTxCountCompleted = quests?.tx_count >= 10;
  const isLimitOrdersCompleted = quests?.limit_orders;
  const isMarketOrdersCompleted = quests?.market_orders;
  const isTradingPairsCompleted = quests?.trading_pairs === 0;
  const isTradingVolumesCompleted = quests?.trading_volumes === 0;

  const areQuestsCompleted =
    quests?.eth_address &&
    isTxCountCompleted &&
    isLimitOrdersCompleted &&
    isMarketOrdersCompleted &&
    isTradingPairsCompleted &&
    isTradingVolumesCompleted;

  if (!isQuestBannerVisible || isLoading) return null;

  return (
    <div className="z-10 w-full shadow-account-card p-4 bg-account-card-blue flex gap-4 flex-col 2xl:flex-row 2xl:items-center justify-between relative">
      <a
        className="exposure-l-italic min-w-fit"
        href="https://app.galxe.com/quest/dango/GCMTJtfErm"
        target="_blank"
        rel="noreferrer"
      >
        {m["quests.galxeQuest.title"]()}
      </a>
      <div className="flex flex-col lg:flex-row w-full justify-between gap-2">
        <div className="flex flex-col lg:flex-row gap-3 px-0 lg:px-4 lg:gap-6 lg:flex-wrap">
          <Quest
            text={`${m["quests.galxeQuest.quest.connectEthereumWallet"]()} ${quests.eth_address ? `(${quests.eth_address})` : ""}`}
            completed={!!quests.eth_address}
          />
          <Quest
            text={m["quests.galxeQuest.quest.swapAtLeastForUSD"]({
              number: formatNumber(
                formatUnits(
                  Decimal(1000000000000)
                    .minus(quests?.trading_volumes || 0)
                    .toFixed(0, 0),
                  6,
                ),
                { ...formatNumberOptions, currency: "USD" },
              ),
            })}
            completed={isTxCountCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest.swapAtLeastInPairs"]({
              number: quests?.trading_pairs,
            })}
            completed={isLimitOrdersCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest.completeLimitOrder"]()}
            completed={isMarketOrdersCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest.completeMarketOrder"]()}
            completed={isTradingPairsCompleted}
          />
          <Quest
            text={m["quests.galxeQuest.quest.completeTxsInEthereum"]()}
            completed={isTradingVolumesCompleted}
          />
        </div>
        {areQuestsCompleted ? (
          <Button
            as="a"
            href="https://app.galxe.com/quest/dango/GCMTJtfErm"
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
