import { useApp } from "@left-curve/foundation";
import { useId } from "react";
import { twMerge } from "@left-curve/foundation";

import { formatNumber } from "@left-curve/dango/utils";

import type React from "react";
import type { FormatNumberOptions } from "@left-curve/dango/utils";

type FormattedNumberProps = {
  number: string | number;
  formatOptions?: Partial<FormatNumberOptions>;
  className?: string;
  as?: "p" | "span";
};

export const FormattedNumber: React.FC<FormattedNumberProps> = ({
  number,
  formatOptions,
  className,
  as = "p",
}) => {
  const id = useId();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const characters = [...formatNumber(number, { ...formatNumberOptions, ...formatOptions })];
  const Component = as;

  return (
    <Component className={twMerge(className)}>
      {characters.map((char, index) => {
        const isNumber = /\d/.test(char);
        return (
          <span
            key={`${id}-char-${
              // biome-ignore lint/suspicious/noArrayIndexKey: better to use index to make sure not repeat the same char
              index
            }`}
            className={isNumber ? "tabular-nums lining-nums" : ""}
          >
            {char}
          </span>
        );
      })}
    </Component>
  );
};
