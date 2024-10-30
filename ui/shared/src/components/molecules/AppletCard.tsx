"use client";

import { useStorage } from "@leftcurve/react";
import type React from "react";

import type { AppletMetadata } from "../../types";
interface Props {
  metadata: AppletMetadata;
  onClick?: (applet: AppletMetadata) => void;
}
export const AppletCard: React.FC<Props> = ({ metadata, onClick }) => {
  const { img, title, description } = metadata;
  const [recentApplets, setRecentApplets] = useStorage<AppletMetadata[]>("applets", {
    initialValue: [],
  });

  const handleOnClick = (applet: AppletMetadata) => {
    onClick?.(applet);

    if (recentApplets.some((applet) => applet.title === metadata.title)) {
      setRecentApplets((applets) => {
        const index = applets.findIndex((applet) => applet.title === metadata.title);
        const recentApplets = Array.from(applets);
        recentApplets.splice(index, 1);
        recentApplets.unshift(metadata);
        return recentApplets;
      });
    } else {
      setRecentApplets((applets) => [metadata, ...applets].slice(0, 4));
    }
  };

  return (
    <div
      className="w-full rounded-3xl bg-surface-rose-200 flex gap-2 cursor-pointer relative group text-black"
      onClick={() => handleOnClick(metadata)}
    >
      <div className="w-[70px] bg-white absolute h-[70px] rounded-3xl group-hover:w-full transition-all" />
      <div className="p-2 relative rounded-3xl flex items-center justify-center z-10">
        <img src={img} alt={title} className="h-[54px] w-[54px]" />
      </div>
      <div className="flex flex-col p-2 relative z-10 flex-1">
        <p className="text-xl font-bold">{title}</p>
        <p className="text-sm text-gray-500">{description}</p>
      </div>
    </div>
  );
};
