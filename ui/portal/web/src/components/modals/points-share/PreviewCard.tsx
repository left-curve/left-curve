import { FormattedNumber } from "@left-curve/applets-kit";
import { getReferralLink, useAccount } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { Ref } from "react";

import { CHARACTERS } from "../../foundation/CharacterSelector";

type PointsCardProps = {
  ref?: Ref<HTMLDivElement>;
  points: number;
  weekNumber: number;
  selectedCharacter: number;
};

export function PreviewCard({ ref, points, weekNumber, selectedCharacter }: PointsCardProps) {
  const { userIndex } = useAccount();

  const referralLink = getReferralLink(userIndex);
  const dangoLogoSrc = "/images/dango.svg";
  const characterImg = `/images/pnl-modal/${CHARACTERS[selectedCharacter]}.png`;

  return (
    <div
      ref={ref}
      className="rounded-2xl shadow-account-card p-6 relative overflow-hidden flex flex-col min-h-[345px] md:w-[47rem] md:h-[26.4375rem]"
      style={{ backgroundColor: "#F5DDB8" }}
    >
      <img src={dangoLogoSrc} alt="Dango" className="relative z-10 h-8 w-auto self-start" />

      <div className="relative z-10 flex-1 flex flex-col justify-center gap-2">
        <p className="exposure-h3-italic text-[#453D39] flex items-center gap-3">
          <span>
            {m["modals.pointsShare.weekLabel"]()} {weekNumber}
          </span>
          <span aria-hidden className="inline-block w-px h-6 bg-[#453D39]" />
          <span>{m["modals.pointsShare.programLabel"]()}</span>
        </p>
        <FormattedNumber
          as="p"
          number={points}
          formatOptions={{ fractionDigits: 0 }}
          className="display-heading-2xl text-[#453D39] leading-none"
        />
      </div>

      {referralLink && (
        <div className="relative z-10">
          <span className="diatype-xs-regular text-[#6E6865]">
            {m["modals.shareCard.referralCode"]()}
          </span>
          <p className="diatype-xs-regular text-[#453D39] break-all">{referralLink}</p>
        </div>
      )}

      <img
        src={characterImg}
        alt="character"
        className="absolute right-0 bottom-0 h-[60%] md:h-full max-h-[9rem] md:max-h-[24rem] opacity-90 pointer-events-none select-none"
      />
    </div>
  );
}
