import { twMerge } from "@left-curve/foundation";
import type React from "react";

type ProgressBarProps = {
  progress: number;
  leftLabel?: string;
  rightLabel?: string;
  thumbSrc?: string;
  endImageSrc?: string;
  endImageAlt?: string;
  className?: string;
  classNames?: {
    container?: string;
    track?: string;
    fill?: string;
    thumb?: string;
    leftLabel?: string;
    rightLabel?: string;
    endImage?: string;
  };
};

const styles = {
  track: "bg-ink-placeholder-400 border border-brand-green",
  fill: "bg-[linear-gradient(321.22deg,_#AFB244_26.16%,_#F9F8EC_111.55%)]",
};

export const ProgressBar: React.FC<ProgressBarProps> = ({
  progress,
  leftLabel,
  rightLabel,
  thumbSrc,
  endImageSrc,
  endImageAlt = "End image",
  className,
  classNames,
}) => {
  const clampedProgress = Math.min(Math.max(progress, 0), 100);

  return (
    <div className={twMerge("w-full flex flex-col gap-2", className, classNames?.container)}>
      <div className="flex items-center gap-3">
        <div
          className={twMerge(
            "relative flex-1 h-3 rounded-full",
            styles.track,
            classNames?.track
          )}
        >
          <div className="absolute inset-0 rounded-full overflow-hidden">
            <div
              className={twMerge(
                "absolute inset-y-0 left-0 rounded-full transition-all duration-300",
                styles.fill,
                classNames?.fill
              )}
              style={{ width: `${clampedProgress}%` }}
            />
          </div>
          {thumbSrc && (
            <img
              src={thumbSrc}
              alt="Progress"
              className={twMerge(
                "absolute -top-3 w-8 h-8 select-none drag-none",
                classNames?.thumb
              )}
              style={{ left: `calc(${clampedProgress}% - 16px)` }}
            />
          )}
        </div>
        {endImageSrc && (
          <img
            src={endImageSrc}
            alt={endImageAlt}
            className={twMerge(
              "w-[4rem] h-auto select-none drag-none hidden lg:block",
              classNames?.endImage
            )}
          />
        )}
        {!endImageSrc && rightLabel && (
          <p
            className={twMerge(
              "diatype-lg-bold text-utility-warning-600",
              classNames?.rightLabel
            )}
          >
            {rightLabel}
          </p>
        )}
      </div>
      {(leftLabel || (endImageSrc && rightLabel)) && (
        <div className="flex justify-between items-center">
          {leftLabel && (
            <p className={twMerge("diatype-m-bold text-ink-tertiary-500", classNames?.leftLabel)}>
              {leftLabel}
            </p>
          )}
          {endImageSrc && rightLabel && (
            <p
              className={twMerge(
                "diatype-lg-bold text-utility-warning-600 ml-auto",
                classNames?.rightLabel
              )}
            >
              {rightLabel}
            </p>
          )}
        </div>
      )}
    </div>
  );
};
