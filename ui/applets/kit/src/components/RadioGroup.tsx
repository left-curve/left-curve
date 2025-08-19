import { useControlledState } from "../hooks/useControlledState.js";
import { useId } from "react";

import { createContext } from "../utils/context.js";
import { twMerge } from "../utils/twMerge.js";

import { motion } from "framer-motion";

import type React from "react";
import type { PropsWithChildren, ReactNode } from "react";

type RadioGroupContextType = {
  name: string;
  value: string | undefined;
  isDisabled?: boolean;
  setValue: (value: string) => void;
};

const [RadioGroupProvider, useRadioGroup] = createContext<RadioGroupContextType>({
  strict: true,
  name: "RadioGroupContext",
});

export type RadioGroupProps = {
  label?: string | ReactNode;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  isDisabled?: boolean;
  error?: string;
};

export const Container: React.FC<PropsWithChildren<RadioGroupProps>> = ({ children }) => {
  return <>{children}</>;
};

export const Group: React.FC<PropsWithChildren<RadioGroupProps>> = ({
  children,
  label,
  value: _value,
  defaultValue,
  onChange,
  isDisabled = false,
  error,
}) => {
  const groupName = useId();
  const [value, setValue] = useControlledState(_value, onChange, defaultValue);

  const context: RadioGroupContextType = {
    name: groupName,
    value,
    isDisabled,
    setValue,
  };

  return (
    <RadioGroupProvider value={context}>
      <div role="radiogroup" aria-labelledby={`${groupName}-label`}>
        {label && (
          <span id={`${groupName}-label`} className="exposure-m-italic text-secondary-700">
            {label}
          </span>
        )}
        <div className="flex flex-col gap-1">{children}</div>
        {error && <p className="text-red-500 text-sm mt-1">{error}</p>}
      </div>
    </RadioGroupProvider>
  );
};

export type RadioProps = {
  value: string;
  label: string | ReactNode;
  isDisabled?: boolean;
  className?: string;
};

export const Item: React.FC<RadioProps> = ({
  value,
  label,
  isDisabled: isDisabledProp = false,
  className,
}) => {
  const ctx = useRadioGroup();
  const isSelected = ctx.value === value;
  const isDisabled = ctx.isDisabled || isDisabledProp;

  return (
    <label
      className={twMerge(
        "flex items-center space-x-2 cursor-pointer transition-opacity",
        isDisabled && "opacity-50 cursor-not-allowed",
        className,
      )}
    >
      <input
        type="radio"
        name={ctx.name}
        value={value}
        checked={isSelected}
        disabled={isDisabled}
        onChange={() => ctx.setValue(value)}
        className="sr-only"
      />
      <motion.div
        initial={false}
        className={twMerge(
          "w-4 h-4 rounded-full flex items-center justify-center transition-all border-2",
          isSelected
            ? "border-red-bean-500 bg-red-bean-500"
            : "border-secondary-gray bg-transparent",
        )}
      >
        {isSelected && (
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{ duration: 0.3 }}
            exit={{ scale: 0 }}
            className="w-[6px] h-[6px] rounded-full bg-white"
          />
        )}
      </motion.div>
      <span className="diatype-sm-medium text-gray-800">{label}</span>
    </label>
  );
};

export const Radio = Object.assign(Container, {
  Group,
  Item,
});
