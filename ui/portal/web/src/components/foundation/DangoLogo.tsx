import { twMerge } from "@left-curve/foundation";
import type React from "react";

type LogoProps = {
  className?: string;
};

export const DangoLogo: React.FC<LogoProps> = ({ className }) => {
  return (
    <img
      src="/dango-logo.svg"
      alt="dango logo"
      className={twMerge(
        "rounded-full shadow-account-card select-none bg-surface-secondary-rice",
        className,
      )}
    />
  );
};
