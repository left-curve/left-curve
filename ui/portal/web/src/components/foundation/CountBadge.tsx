import type React from "react";

type CountBadgeProps = {
  count: number;
};

export const CountBadge: React.FC<CountBadgeProps> = ({ count }) => {
  if (count <= 0) return null;
  return (
    <span className="bg-red-500 text-surface-primary-rice rounded-full min-w-[22px] h-[22px] flex items-center justify-center diatype-sm-medium not-italic pt-0.5 px-2">
      {count}
    </span>
  );
};
