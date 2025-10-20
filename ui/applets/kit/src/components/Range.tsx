import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useControlledState } from "@left-curve/foundation";

import { Input } from "./Input";

import { twMerge } from "@left-curve/foundation";

import type React from "react";

const clampValueToStep = (value: number, min: number, max: number, step: number): number => {
  const valueRelativeToMin = value - min;
  let steppedValue = Math.round(valueRelativeToMin / step) * step + min;
  const precision = step.toString().split(".")[1]?.length || 0;
  steppedValue = Number.parseFloat(steppedValue.toFixed(precision));
  return Math.max(min, Math.min(max, steppedValue));
};

const formatInputValue = (num: number, precision: number): string => {
  if (num % 1 === 0) {
    return num.toFixed(0);
  }
  return num.toFixed(precision);
};

export function getDynamicStep(maxValue: number, explicitStep?: number): number {
  if (typeof explicitStep === "number" && explicitStep > 0) return explicitStep;
  if (!Number.isFinite(maxValue) || maxValue <= 0) return 0.1;

  const idealStep = maxValue / 120;
  const power = 10 ** Math.floor(Math.log10(idealStep));
  const base = idealStep / power;
  const mult = base < 1.5 ? 1 : base < 3.5 ? 2 : base < 7.5 ? 5 : 10;
  const step = Math.max(mult * power, 1e-6);

  return Number(step.toPrecision(2));
}

export type StepObject = { value: number; label: string };

export type RangeProps = {
  minValue: number;
  maxValue: number;
  step?: number;
  defaultValue?: number;
  value?: number;
  onChange?: (value: number) => void;
  label?: string | ReactNode;
  isDisabled?: boolean;
  showSteps?: boolean | StepObject[];
  showPercentage?: boolean;
  classNames?: {
    base?: string;
    input?: string;
    inputWrapper?: string;
  };
  withInput?: boolean;
  inputEndContent?: ReactNode;
};

export const Range: React.FC<RangeProps> = ({
  minValue,
  maxValue,
  defaultValue,
  value: controlledValue,
  onChange,
  label,
  isDisabled = false,
  showSteps = false,
  classNames,
  withInput = false,
  inputEndContent,
  showPercentage = false,
}) => {
  const stepToUse = useMemo(() => getDynamicStep(maxValue), [maxValue]);

  const [value, setValue] = useControlledState(controlledValue, onChange, () => {
    const initial = defaultValue !== undefined ? defaultValue : minValue;
    return clampValueToStep(initial, minValue, maxValue, stepToUse);
  });

  const sliderRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);

  const getPercentage = useCallback(
    (val: number) => {
      if (maxValue === minValue) return 0;
      return Math.min(((val - minValue) / (maxValue - minValue)) * 100, 100);
    },
    [minValue, maxValue],
  );

  const currentPercentage = getPercentage(value);

  const handleInteraction = useCallback(
    (clientX: number) => {
      if (!sliderRef.current) return;
      const trackRect = sliderRef.current.getBoundingClientRect();
      const clickPos = clientX - trackRect.left;
      const newValueRatio = Math.max(0, Math.min(1, clickPos / trackRect.width));
      const newValue = minValue + newValueRatio * (maxValue - minValue);
      const clamped = clampValueToStep(newValue, minValue, maxValue, stepToUse);
      setValue(clamped);
    },
    [minValue, maxValue, stepToUse, setValue],
  );

  const handleSliderMouseDown = useCallback(
    (event: React.MouseEvent<HTMLDivElement> | React.TouchEvent<HTMLDivElement>) => {
      if (isDisabled) return;
      event.preventDefault();
      setIsDragging(true);
      const clientX = "touches" in event ? event.touches[0].clientX : event.clientX;
      handleInteraction(clientX);
    },
    [isDisabled, handleInteraction],
  );

  useEffect(() => {
    const handleMouseMove = (event: MouseEvent | TouchEvent) => {
      if (!isDragging || isDisabled) return;
      const clientX = "touches" in event ? event.touches[0].clientX : event.clientX;
      handleInteraction(clientX);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    if (isDragging) {
      window?.document.addEventListener("mousemove", handleMouseMove);
      window?.document.addEventListener("touchmove", handleMouseMove, { passive: false });
      window?.document.addEventListener("mouseup", handleMouseUp);
      window?.document.addEventListener("touchend", handleMouseUp);
    }

    return () => {
      window?.document.removeEventListener("mousemove", handleMouseMove);
      window?.document.removeEventListener("touchmove", handleMouseMove);
      window?.document.removeEventListener("mouseup", handleMouseUp);
      window?.document.removeEventListener("touchend", handleMouseUp);
    };
  }, [isDragging, isDisabled, handleInteraction]);

  const handleThumbKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (isDisabled) return;
    let newValue = value;
    let valueChanged = true;
    switch (event.key) {
      case "ArrowLeft":
      case "ArrowDown":
        newValue = value - stepToUse;
        break;
      case "ArrowRight":
      case "ArrowUp":
        newValue = value + stepToUse;
        break;
      case "PageDown":
        newValue = value - stepToUse * 10;
        break;
      case "PageUp":
        newValue = value + stepToUse * 10;
        break;
      case "Home":
        newValue = minValue;
        break;
      case "End":
        newValue = maxValue;
        break;
      default:
        valueChanged = false;
        break;
    }

    if (valueChanged) {
      event.preventDefault();
      const clampedNewValue = clampValueToStep(newValue, minValue, maxValue, stepToUse);
      setValue(clampedNewValue);
    }
  };

  const stepsToDisplay: StepObject[] = [];
  if (showSteps) {
    if (Array.isArray(showSteps)) {
      stepsToDisplay.push(...showSteps);
    } else {
      stepsToDisplay.push({ value: minValue, label: `${minValue}${withInput ? "x" : ""}` });
      const midPoint = Number.parseFloat(
        ((minValue + maxValue) / 2).toFixed(stepToUse.toString().split(".")[1]?.length || 0),
      );
      if (
        midPoint !== minValue &&
        midPoint !== maxValue &&
        midPoint > minValue &&
        midPoint < maxValue
      ) {
        stepsToDisplay.push({ value: midPoint, label: `${midPoint}${withInput ? "x" : ""}` });
      }
      if (minValue !== maxValue) {
        stepsToDisplay.push({ value: maxValue, label: `${maxValue}${withInput ? "x" : ""}` });
      }
    }
  }

  const inputPrecision = Math.max(
    0,
    stepToUse.toString().split(".")[1]?.length || 0,
    minValue.toString().split(".")[1]?.length || 0,
    maxValue.toString().split(".")[1]?.length || 0,
  );

  return (
    <div
      className={twMerge("w-full flex flex-col mt-1", { "gap-3": !withInput }, classNames?.base)}
    >
      {label && <div className="text-ink-tertiary-500 exposure-xs-italic">{label}</div>}

      <div className="flex items-center gap-3">
        <div
          className={twMerge(
            "flex flex-col flex-1",
            { "mt-4": showPercentage },
            classNames?.inputWrapper,
          )}
        >
          <div
            ref={sliderRef}
            className={twMerge(
              "relative h-1 rounded-full",
              isDisabled ? "bg-surface-disabled-gray" : "bg-outline-secondary-gray cursor-pointer",
            )}
            onMouseDown={handleSliderMouseDown}
            onTouchStart={handleSliderMouseDown}
          >
            <div
              className={twMerge(
                "absolute top-0 left-0 h-full rounded-full",
                isDisabled ? "bg-primitives-gray-light-400" : "bg-primitives-red-light-400",
              )}
              style={{ width: `${currentPercentage}%` }}
            />

            <div
              className={twMerge(
                "absolute top-1/2 w-4 h-4 rounded-full shadow-md focus:outline-none focus:border-primitives-red-light-600",
                isDisabled
                  ? "bg-primitives-gray-light-300 border-2 border-primitives-gray-light-500"
                  : "bg-white border-2 border-primitives-red-light-500 cursor-grab active:cursor-grabbing",
              )}
              style={{
                left: `calc(${currentPercentage}% - ${currentPercentage < 2 ? "0px" : "16px"})`,
                transform: "translateY(-50%)",
              }}
              tabIndex={isDisabled ? -1 : 0}
              role="slider"
              aria-valuemin={minValue}
              aria-valuemax={maxValue}
              aria-valuenow={value}
              aria-disabled={isDisabled}
              aria-label={typeof label === "string" ? label : "Slider value"}
              onMouseDown={(e) => {
                e.stopPropagation();
                if (!isDisabled) setIsDragging(true);
              }}
              onTouchStart={(e) => {
                e.stopPropagation();
                if (!isDisabled) setIsDragging(true);
              }}
              onKeyDown={handleThumbKeyDown}
            >
              {showPercentage && (
                <p className="absolute -top-5 text-ink-tertiary-500 exposure-xs-italic select-none">
                  {currentPercentage.toFixed(0)}%
                </p>
              )}
            </div>
          </div>
          {showSteps && stepsToDisplay.length > 0 && (
            <div className="flex justify-between mt-2 px-1">
              {stepsToDisplay.map((s) => {
                return (
                  <span
                    key={`stepper-${s.value}`}
                    className="text-ink-tertiary-500 diatype-xs-regular cursor-pointer"
                    onClick={() => setValue(s.value)}
                  >
                    {s.label}
                  </span>
                );
              })}
            </div>
          )}
        </div>

        {withInput && (
          <Input
            value={formatInputValue(value, inputPrecision)}
            min={minValue}
            max={maxValue}
            step={stepToUse}
            disabled={isDisabled}
            placeholder={minValue.toString()}
            endContent={
              inputEndContent ? (
                <span className="text-ink-tertiary-500">{inputEndContent}</span>
              ) : null
            }
            onChange={(e) => {
              const rawValue = e.target.value;
              const value = rawValue === "" ? "0" : rawValue;
              const numValue = Number.parseFloat(value);
              if (!Number.isNaN(numValue)) {
                setValue(numValue);
              }
            }}
            classNames={{ base: twMerge("max-w-[5rem]", classNames?.input) }}
          />
        )}
      </div>
    </div>
  );
};
