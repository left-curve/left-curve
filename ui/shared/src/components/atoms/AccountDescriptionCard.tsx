"use client";
import type React from "react";

interface Props extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  img: string;
  description: string;
}

export const AccountDescriptionCard: React.FC<Props> = ({ title, img, description, ...props }) => {
  return (
    <div
      className="w-full p-3 md:p-4 flex gap-6 rounded-2xl bg-surface-purple-200 hover:bg-surface-rose-200 transition-all cursor-pointer"
      {...props}
    >
      <img src={img} alt={title} className="w-20 h-20" />
      <div className="flex flex-col gap-2">
        <h3 className="font-bold text-typography-black-200 font-diatype-rounded tracking-widest uppercase">
          {title}
        </h3>
        <p className="text-typography-black-100 text-xs">{description}</p>
      </div>
    </div>
  );
};
