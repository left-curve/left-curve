import type React from "react";
import { twMerge } from "../../utils";

export const Skeleton: React.FC<{ className?: string }> = ({ className }) => {
  return <div className={twMerge("animate-pulse bg-white-400/70 rounded-md", className)} />;
};
