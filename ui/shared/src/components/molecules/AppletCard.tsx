"use client";

import type React from "react";

import type { AppletMetadata } from "../../types";
import { Emoji, type EmojiName } from "../atoms/Emoji";

interface Props {
  metadata: AppletMetadata;
}

export const AppletCard: React.FC<Props> = ({ metadata }) => {
  const { img, title, description } = metadata;

  return (
    <div className="applet-card w-full rounded-[1.25rem] bg-surface-rose-200 flex gap-2 cursor-pointer relative text-black my-2">
      <div className="w-[100px] bg-surface-off-white-200 absolute min-h-[80px] rounded-l-[1.25rem] rounded-r-[3.5rem] ] group-data-[selected=true]:w-full group-data-[selected=true]:rounded-[1.25rem] transition-all" />
      <div className="py-2 pl-4 pr-5 relative rounded-[1.25rem] flex items-center justify-center z-10">
        <Emoji name={img as EmojiName} className="h-[54px] w-[54px]" />
      </div>
      <div className="flex flex-col px-5 py-4 relative z-10 flex-1 overflow-hidden">
        <p className="sm:typography-body-l !font-bold">{title}</p>
        <p className="sm:typography-body-m text-gray-500 truncate">{description}</p>
      </div>
    </div>
  );
};
