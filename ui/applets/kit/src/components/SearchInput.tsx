import { useQuery } from "@tanstack/react-query";
import { type ReactElement, cloneElement, forwardRef, useState } from "react";
import { useClickAway } from "react-use";
import { useDOMRef } from "../hooks/useDOMRef.js";
import { useControlledState } from "../hooks/useControlledState.js";

import { twMerge } from "@left-curve/foundation";
import { wait } from "@left-curve/dango/utils";

import { Input, type InputProps } from "./Input";
import { Spinner } from "./Spinner";

export interface SearchInputProps extends Omit<InputProps, "value" | "defaultValue" | "onChange"> {
  value?: string;
  defaultValue?: string;
  onChange?: (newValue: string) => void;
  getOptionsFn?: (inputValue: string) => Promise<{ key: string; value: string }[]>;
  optComponent?: React.ReactElement;
}

export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(
  ({ value, onChange, defaultValue, getOptionsFn, optComponent, ...props }, ref) => {
    const menuRef = useDOMRef<HTMLDivElement>(null);
    const [showMenu, setShowMenu] = useState(false);

    const [inputValue, setInputValue] = useControlledState(value, onChange, defaultValue ?? "");

    const OptionComponent = optComponent || <p />;

    const { data: options, isFetching } = useQuery({
      enabled: inputValue.length > 0,
      queryKey: ["search_input_opts", inputValue],
      queryFn: async ({ signal }) => {
        await wait(300);
        if (signal.aborted) return [];
        if (!getOptionsFn) return [];
        const options = await getOptionsFn?.(inputValue);
        if (!options.length) return [];
        setShowMenu(true);
        return options;
      },
      initialData: [],
    });

    useClickAway(menuRef, () => setShowMenu(false));

    return (
      <div className="relative">
        <Input
          {...props}
          ref={ref}
          value={inputValue}
          data-1p-ignore
          onClick={() => setShowMenu(true)}
          onChange={(e) => setInputValue(e.target.value)}
          endContent={isFetching ? <Spinner size="sm" color="gray" /> : null}
          classNames={{
            inputWrapper: showMenu && options.length ? "rounded-b-none" : "",
          }}
          {...props}
        />
        <div
          ref={menuRef}
          className={twMerge(
            "absolute top-[4.8rem] shadow-account-card bg-surface-secondary-rice rounded-lg p-1 z-30 w-full",
            showMenu ? "block" : "hidden",
            options.length ? "scale-100" : "scale-0",
          )}
        >
          {options.map(({ key, value }) => (
            <button
              onClick={() => [setInputValue(value), setShowMenu(false)]}
              type="button"
              className="w-full p-3 hover:bg-surface-tertiary-rice rounded-md text-left"
              key={key}
            >
              {cloneElement(OptionComponent as ReactElement, { children: key })}
            </button>
          ))}
        </div>
      </div>
    );
  },
);
