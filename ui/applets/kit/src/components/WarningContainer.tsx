import { twMerge } from "@left-curve/foundation";
import { IconInfo } from "./icons/IconInfo";

import type React from "react";

interface WarningContainerProps {
  title?: string;
  description?: React.ReactNode;
  extraContent?: React.ReactNode;
  color?: "warning" | "error";
  className?: string;
}

const COLOR_STYLES = {
  warning: {
    container: "bg-utility-warning-100 text-ink-tertiary-500",
    icon: "text-utility-warning-600",
  },
  error: {
    container: "bg-surface-secondary-red text-ink-secondary-red border border-outline-primary-red",
    icon: "text-ink-secondary-red",
  },
} as const;

export const WarningContainer: React.FC<WarningContainerProps> = ({
  title,
  description,
  extraContent,
  color = "warning",
  className,
}) => {
  const styles = COLOR_STYLES[color];
  return (
    <div
      className={twMerge(
        "rounded-xl diatype-sm-regular py-2 px-3 flex gap-2",
        styles.container,
        className,
      )}
    >
      <IconInfo className={`w-6 h-6 ${styles.icon}`} />
      <div className="flex-1 w-full flex flex-col gap-1">
        {title && <p className="flex-1 w-full diatype-sm-bold text-ink-primary-900">{title}</p>}
        {description && <div>{description}</div>}
        {extraContent && extraContent}
      </div>
    </div>
  );
};
