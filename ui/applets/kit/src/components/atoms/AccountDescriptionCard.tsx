"use client";
import type React from "react";
import { twMerge } from "../../utils";

interface Props extends React.HTMLAttributes<HTMLDivElement> {
  title: string;
  img: string;
  description: string;
}

export const AccountDescriptionCard: React.FC<Props> = ({
  title,
  img,
  description,
  className,
  ...props
}) => {
  return (
    <div
      className={twMerge(
        "w-full p-3 md:p-4 flex gap-6 rounded-2xl transition-all cursor-pointer",
        className,
      )}
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
