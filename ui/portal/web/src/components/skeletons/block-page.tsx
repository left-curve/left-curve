import { Skeleton } from "@left-curve/applets-kit";
import type React from "react";

export const BlockPageSkeleton: React.FC = () => {
  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden md:min-h-[147.22px] min-h-[208.5px]">
        <h1 className="h4-bold">Block Detail</h1>
        <Skeleton className="h-full w-full max-w-[75%]" />
        <img
          src="/images/emojis/detailed/map-explorer.svg"
          alt="map-emoji"
          className="hidden md:block w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply"
        />
      </div>
    </div>
  );
};
