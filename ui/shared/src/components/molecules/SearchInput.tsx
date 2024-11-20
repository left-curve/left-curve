"use client";

import { useControlledState } from "@react-stately/utils";
import { useQuery } from "@tanstack/react-query";
import { type ReactElement, cloneElement, useState } from "react";
import { useClickAway, useDebounce } from "react-use";
import { forwardRef, twMerge, useDOMRef } from "../../utils";

import { Input, type InputProps } from "../atoms/Input";
import { Spinner } from "../atoms/Spinner";

export interface SearchInputProps extends Omit<InputProps, "value" | "defaultValue" | "onChange"> {
  value?: string;
  defaultValue?: string;
  onChange?: (newValue: string) => void;
  getOptionsFn?: (inputValue: string) => Promise<string[]>;
  optComponent?: React.ReactElement;
}

export const SearchInput = forwardRef<"input", SearchInputProps>(
  ({ value, onChange, defaultValue, getOptionsFn, optComponent, ...props }, ref) => {
    const menuRef = useDOMRef<HTMLDivElement>(null);
    const [showMenu, setShowMenu] = useState(false);

    const [inputValue, setInputValue] = useControlledState(value, defaultValue ?? "", onChange);

    const OptionComponent = optComponent || <p />;

    const {
      refetch,
      data: options,
      isFetching,
    } = useQuery({
      enabled: false,
      queryKey: ["search_input_opts", inputValue],
      queryFn: async () => {
        if (!getOptionsFn) return [];
        const options = await getOptionsFn?.(inputValue);
        if (!options.length) return [];
        setShowMenu(true);
        return options;
      },
      initialData: [],
    });

    useDebounce(refetch, 300, [inputValue]);
    useClickAway(menuRef, () => setShowMenu(false));

    return (
      <div className="relative">
        <Input
          ref={ref}
          value={inputValue}
          disabled={isFetching}
          onClick={() => setShowMenu(true)}
          onChange={(e) => setInputValue(e.target.value)}
          endContent={isFetching ? <Spinner size="sm" color="white" /> : null}
          classNames={{
            inputWrapper: showMenu && options.length ? "rounded-b-none" : "",
          }}
          {...props}
        />

        <div
          ref={menuRef}
          className={twMerge(
            "absolute flex flex-col gap-2 overflow-hidden bg-surface-rose-300 w-full min-h-10 z-20 p-2 rounded-b-xl",
            showMenu ? "block" : "hidden",
            options.length ? "scale-100" : "scale-0",
          )}
        >
          {options.map((option: string) => (
            <button
              onClick={() => [setInputValue(option), setShowMenu(false)]}
              type="button"
              className="w-full text-typography-rose-600 hover:bg-surface-rose-400 rounded-xl text-start p-4"
              key={option}
            >
              {cloneElement(OptionComponent as ReactElement, { children: option })}
            </button>
          ))}
        </div>
      </div>
    );
  },
);
