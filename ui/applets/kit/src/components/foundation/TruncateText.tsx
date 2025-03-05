import type { ComponentPropsWithoutRef } from "react";
import { twMerge } from "../../utils";

type Props = {
  text?: string;
  children?: string;
  start?: number;
  end?: number;
};

export const TruncateText: React.FC<Props & ComponentPropsWithoutRef<"p">> = ({
  children,
  text,
  className,
  start,
  end,
  ...props
}) => {
  const slot = children ? children : text ? text : "";
  return (
    <p className={twMerge("flex overflow-auto", className)} {...props}>
      <span className="truncate">{slot.slice(0, start || 8)}</span>
      <span>...</span>
      <span>{slot.slice(slot.length - (end || 8))}</span>
    </p>
  );
};

export default TruncateText;
