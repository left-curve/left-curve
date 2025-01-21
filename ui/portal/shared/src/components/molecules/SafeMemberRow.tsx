import type { Power, Username } from "@left-curve/types";

interface Props {
  username: Username;
  power: Power;
  totalPower: number;
}

export const SafeMemberRow: React.FC<Props> = ({ username, power, totalPower }) => {
  return (
    <div className="p-2 md:p-4 rounded-2xl flex items-center justify-between  bg-surface-yellow-200 w-full">
      <div>{username}</div>
      <div className="flex items-center justify-center gap-2 text-bold text-lg">
        <p>
          {power} ({(power * 100) / totalPower}%)
        </p>
      </div>
    </div>
  );
};
