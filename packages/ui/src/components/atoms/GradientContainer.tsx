"use client";

import type React from "react";

import type { ComponentPropsWithoutRef } from "react";
import { twMerge } from "~/utils";

export const GradientContainer: React.FC<ComponentPropsWithoutRef<"div">> = ({
  className,
  children,
}) => {
  return (
    <div
      className={twMerge(
        "bg-gradient-container backdrop-blur-xl rounded-3xl flex flex-col gap-3 items-center justify-between text-sand-900 p-4 h-fit w-fit",
        className,
      )}
    >
      {children}
    </div>
  );
};
