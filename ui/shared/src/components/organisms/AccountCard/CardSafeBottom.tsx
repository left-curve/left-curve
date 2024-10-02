import type { Power, Username } from "@leftcurve/types";
import { AvatarStack } from "~/components/atoms/AvatarStack";

interface Props {
  isLoading: boolean;
  totalBalance: string;
  members: Record<Username, Power>;
}

export const CardSafeBottom: React.FC<Props> = ({ isLoading, totalBalance, members }) => {
  const images = Object.keys(members).map(
    (username) => `https://www.tapback.co/api/avatar/${username}.webp`,
  );
  return (
    <div className="flex flex-col flex-start gap-1">
      <AvatarStack images={images} className="max-h-8" />
      <div className="flex items-center justify-between">
        <p className="uppercase text-[10px] text-typography-yellow-300 font-semibold">balance:</p>
        <p className="text-sm font-extrabold text-typography-yellow-400">
          {isLoading ? "0" : totalBalance}
        </p>
      </div>
    </div>
  );
};
