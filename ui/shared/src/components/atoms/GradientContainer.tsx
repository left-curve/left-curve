"use client";

import type React from "react";

import type { ComponentPropsWithoutRef } from "react";
import { twMerge } from "../../utils";

export const GradientContainer: React.FC<ComponentPropsWithoutRef<"div">> = ({
  className,
  children,
}) => {
  return (
    <div
      className={twMerge(
        "backdrop-blur-xl flex flex-col items-center justify-between w-fit h-fit",
        className,
      )}
    >
      {children}
    </div>
  );
};
