import { useControlledState } from "@left-curve/foundation";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { forwardRef, useState } from "react";

import { useDOMRef } from "../hooks/useDOMRef.js";
import { useClickAway } from "../hooks/useClickAway.js";

import { capitalize, wait } from "@left-curve/dango/utils";
import { twMerge } from "@left-curve/foundation";

import { Input, type InputProps } from "./Input";
import { Spinner } from "./Spinner";
import TruncateText from "./TruncateText";

import type { Prettify } from "@left-curve/dango/types";

type AccountSearchInputProps = Prettify<
  InputProps & {
    value?: string;
    defaultValue?: string;
    onChange?: (value: string) => void;
  }
>;

export const AccountSearchInput = forwardRef<HTMLInputElement, AccountSearchInputProps>(
  (props, ref) => {
    const { value, onChange, defaultValue } = props;
    const client = usePublicClient();

    const menuRef = useDOMRef<HTMLDivElement>(null);
    const [showMenu, setShowMenu] = useState(false);

    const [inputValue, setInputValue] = useControlledState(value, onChange, defaultValue ?? "");

    const { data: options, isFetching } = useQuery({
      enabled: inputValue.length > 0,
      queryKey: ["search_input_opts", inputValue],
      queryFn: async ({ signal }) => {
        await wait(300);
        if (signal.aborted) return [];
        const { accounts } = await client.getUser({ username: inputValue });

        const options = Object.entries(accounts).map(([address, account]) => ({
          address,
          accountName: `${capitalize(Object.keys(account.params).at(0) as string)} Account #${account.index}`,
        }));

        if (!options) return [];
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
          onChange={(e) => [setInputValue(e.target.value), onChange?.(e)]}
          endContent={isFetching ? <Spinner size="sm" color="gray" /> : null}
          {...props}
        />
        <div
          ref={menuRef}
          className={twMerge(
            "absolute top-[4.8rem] shadow-account-card bg-surface-secondary-rice rounded-lg p-1 z-30 w-full overflow-y-scroll max-h-[20rem] transition-all duration-300 ease-in-out scrollbar-none",
            showMenu ? "block" : "hidden",
            options.length
              ? "h-auto translate-x-0"
              : "h-0 -translate-x-30 overflow-hidden py-0 px-1",
          )}
        >
          <p className="diatype-sm-medium text-tertiary-500 px-3 pt-2">Accounts</p>
          {options.map(({ accountName, address }) => (
            <div
              onClick={() => [setInputValue(address), setShowMenu(false)]}
              className="w-full px-3 py-2 hover:bg-surface-tertiary-rice rounded-md text-left cursor-pointer"
              key={address}
            >
              <div className="flex items-center gap-4">
                <div className="p-1 bg-primary-red rounded-xxs border border-surface-secondary-red w-fit">
                  <img
                    src="/images/emojis/simple/hamster.svg"
                    alt={address}
                    className="w-12 h-12"
                  />
                </div>
                <div className="w-fit flex flex-col gap-1 overflow-x-hidden">
                  <p className="diatype-lg-medium">{accountName}</p>
                  <TruncateText
                    className="diatype-m-regular text-tertiary-500"
                    text={address}
                    start={20}
                  />
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  },
);
