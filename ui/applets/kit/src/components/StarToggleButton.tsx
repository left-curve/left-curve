import { memo } from "react";
import { twMerge } from "@left-curve/foundation";

import { IconStar } from "./icons/IconStar";
import { IconEmptyStar } from "./icons/IconEmptyStar";

type StarToggleButtonProps = {
  isActive: boolean;
  onToggle: () => void;
  className?: string;
  isDisabled?: boolean;
  "aria-label"?: string;
};

export const StarToggleButton = memo<StarToggleButtonProps>(
  ({ isActive, onToggle, className, isDisabled, "aria-label": ariaLabel }) => {
    const Icon = isActive ? IconStar : IconEmptyStar;
    return (
      <button
        type="button"
        aria-label={ariaLabel ?? (isActive ? "Remove from favorites" : "Add to favorites")}
        aria-pressed={isActive}
        disabled={isDisabled}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => {
          e.stopPropagation();
          onToggle();
        }}
        className="focus:outline-none flex-shrink-0 disabled:opacity-50"
      >
        <Icon className={twMerge("w-4 h-4 text-fg-primary-700", className)} />
      </button>
    );
  },
);
