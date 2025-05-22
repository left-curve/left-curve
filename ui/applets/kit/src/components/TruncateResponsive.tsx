import { twMerge } from "#utils/twMerge.js";
import type React from "react";
import { useMemo } from "react";

type TruncateResponsiveProps = {
  text: string;
  lastNumbers?: number;
  className?: string;
};

export const TruncateResponsive: React.FC<TruncateResponsiveProps> = ({
  text,
  lastNumbers = 6,
  className,
}) => {
  const { start, end } = useMemo(() => {
    const visibleEnd = text.slice(-lastNumbers);
    const hiddenStart = text.slice(0, text.length - lastNumbers);
    return {
      start: hiddenStart,
      end: visibleEnd,
    };
  }, [text, lastNumbers]);

  return (
    <span className={twMerge("flex overflow-hidden", className)}>
      <span className="truncate">{start}</span>
      <span>{end}</span>
    </span>
  );
};
