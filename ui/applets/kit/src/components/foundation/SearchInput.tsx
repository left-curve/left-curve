import { useQuery } from "@tanstack/react-query";
import { type ReactElement, cloneElement, useState } from "react";
import { useClickAway } from "react-use";
import { forwardRef, twMerge, useDOMRef } from "../../utils";

import { wait } from "@left-curve/dango/utils";
import { useControlledState } from "../../hooks";
import { Input, type InputProps } from "./Input";
import { Spinner } from "./Spinner";

export interface SearchInputProps extends Omit<InputProps, "value" | "defaultValue" | "onChange"> {
  value?: string;
  defaultValue?: string;
  onChange?: (newValue: string) => void;
  getOptionsFn?: (inputValue: string) => Promise<{ key: string; value: string }[]>;
  optComponent?: React.ReactElement;
}

export const SearchInput = forwardRef<"input", SearchInputProps>(
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
            "absolute bottom-[-3.8rem] shadow-card-shadow bg-rice-25 rounded-lg p-1 z-30 w-full",
            showMenu ? "block" : "hidden",
            options.length ? "scale-100" : "scale-0",
          )}
        >
          {options.map(({ key, value }) => (
            <button
              onClick={() => [setInputValue(value), setShowMenu(false)]}
              type="button"
              className="w-full p-3 hover:bg-rice-50 rounded-md text-left"
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
