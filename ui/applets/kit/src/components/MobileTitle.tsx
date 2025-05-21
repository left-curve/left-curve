import type React from "react";
import { IconButton } from "./IconButton";
import { IconChevronDown } from "./icons/IconChevronDown";
import { twMerge } from "#utils/twMerge.js";

interface Props {
  action: () => void;
  title: string;
  className?: string;
}

export const MobileTitle: React.FC<Props> = ({ action, title, className }) => {
  return (
    <div className={twMerge("flex gap-2 items-center lg:hidden", className)} onClick={action}>
      <IconButton variant="link">
        <IconChevronDown className="rotate-90" />
      </IconButton>

      <h2 className="h3-bold text-gray-900">{title}</h2>
    </div>
  );
};
