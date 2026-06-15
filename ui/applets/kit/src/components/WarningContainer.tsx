import { twMerge } from "@left-curve/foundation";
import { IconInfo } from "./icons/IconInfo";

import type React from "react";

interface WarningContainerProps {
  title?: string;
  description?: React.ReactNode;
  extraContent?: React.ReactNode;
  color?: "success" | "warning" | "error";
  className?: string;
}

const COLOR_STYLES = {
  success: {
    container: "bg-surface-alert-success text-fg-alert-success",
    icon: "text-fg-alert-success",
  },
  warning: {
    container: "bg-surface-alert-warning text-fg-alert-warning",
    icon: "text-fg-alert-warning",
  },
  error: {
    container: "bg-surface-alert-error text-fg-alert-error",
    icon: "text-fg-alert-error",
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
        "rounded-xl shadow-account-card diatype-sm-regular py-2 px-3 flex gap-2",
        styles.container,
        className,
      )}
    >
      <IconInfo className={`w-6 h-6 ${styles.icon}`} />
      <div className="flex-1 w-full flex flex-col gap-1">
        {title && <p className="flex-1 w-full diatype-sm-bold">{title}</p>}
        {description && <div>{description}</div>}
        {extraContent && extraContent}
      </div>
    </div>
  );
};
