import { IconInfo } from "./icons/IconInfo";

import type React from "react";

interface WarningContainerProps {
  title?: string;
  description?: React.ReactNode;
  extraContent?: React.ReactNode;
}

export const WarningContainer: React.FC<WarningContainerProps> = ({
  title,
  description,
  extraContent,
}) => {
  return (
    <div className="rounded-xl bg-utility-warning-100 text-ink-tertiary-500 diatype-sm-regular py-2 px-3 flex gap-2">
      <IconInfo className="w-6 h-6 text-utility-warning-600" />
      <div className="flex-1 w-full flex flex-col gap-1">
        {title && <p className="flex-1 w-full diatype-sm-bold text-ink-primary-900">{title}</p>}
        {description && <div>{description}</div>}
        {extraContent && extraContent}
      </div>
    </div>
  );
};
