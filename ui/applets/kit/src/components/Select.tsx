import {
  Children,
  createContext,
  isValidElement,
  useContext,
  useId,
  useRef,
  useState,
} from "react";
import { useClickAway } from "react-use";
import { useControlledState } from "#hooks/index.js";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDown } from "./icons/IconChevronDown";

import { tv } from "tailwind-variants";
import { twMerge } from "#utils/index.js";

import type { PropsWithChildren, ReactElement } from "react";
import type React from "react";
import type { VariantProps } from "tailwind-variants";

const SelectContext = createContext<{
  selected: string;
  setSelected: (val: string) => void;
} | null>(null);

const useSelectContext = () => {
  const context = useContext(SelectContext);
  if (!context) {
    throw new Error("Select components cannot be rendered outside the Select component");
  }
  return context;
};

export interface SelectProps extends VariantProps<typeof selectVariants> {
  defaultValue?: string;
  onChange?: (value: string) => void;
  value?: string;
  classNames?: {
    base?: string;
    listboxWrapper?: string;
    listbox?: string;
    value?: string;
    trigger?: string;
    icon?: string;
  };
}

const Root: React.FC<PropsWithChildren<SelectProps>> = (props) => {
  const { classNames, children, onChange, value, defaultValue, isDisabled } = props;

  const selectRef = useRef<HTMLDivElement>(null);

  const [isOpen, setIsOpen] = useState(false);
  const [selected, setSelected] = useControlledState(
    value,
    (v) => {
      onChange?.(v);
      setIsOpen(false);
    },
    defaultValue,
  );

  const { base, trigger, listboxWrapper, icon } = selectVariants({ isDisabled });

  useClickAway(selectRef, () => setIsOpen(false));

  return (
    <SelectContext.Provider value={{ selected, setSelected }}>
      <div className={base({ className: classNames?.base })}>
        <NativeSelect classNames={classNames}>{children}</NativeSelect>

        <div className="hidden md:block relative w-full" ref={selectRef}>
          <button
            type="button"
            onClick={() => !isDisabled && setIsOpen((prev) => !prev)}
            className={trigger({ className: classNames?.trigger })}
          >
            <span>
              {
                (
                  Children.toArray(children).find(
                    (e) => isValidElement(e) && selected === (e as ReactElement).props.value,
                  ) as { props: { children: ReactElement } }
                )?.props.children
              }
            </span>
            <IconChevronDown className={icon({ className: classNames?.icon })} />
          </button>

          <motion.div
            layout="size"
            className={listboxWrapper({
              className: classNames?.listboxWrapper,
            })}
          >
            <AnimatePresence>
              {isOpen && (
                <motion.div
                  style={{ overflow: "hidden" }}
                  initial={{ height: 0 }}
                  animate={{ transition: { duration: 0.1 }, height: isOpen ? "auto" : 0 }}
                  exit={{ height: 0 }}
                >
                  <motion.ul
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.05 }}
                    className={twMerge(
                      "w-full max-h-[12rem] p-2 outline-none gap-1 flex flex-col scrollbar-none overflow-auto",
                    )}
                  >
                    {children}
                  </motion.ul>
                </motion.div>
              )}
            </AnimatePresence>
          </motion.div>
        </div>
      </div>
    </SelectContext.Provider>
  );
};

type SelectItemProps = {
  value: string;
};

const Item: React.FC<PropsWithChildren<SelectItemProps>> = ({ value, children }) => {
  const { setSelected } = useSelectContext();

  return (
    <li
      value={value}
      onClick={() => setSelected(value)}
      className={twMerge(
        "rounded-sm py-2 px-3 text-base outline-none cursor-pointer flex items-center transition-all diatype-m-medium bg-rice-25 leading-none hover:bg-rice-50",
      )}
    >
      {children}
    </li>
  );
};

type NativeSelectProps = {
  classNames?: {
    base?: string;
    trigger?: string;
  };
};

export const NativeSelect: React.FC<PropsWithChildren<NativeSelectProps>> = ({
  children,
  classNames,
}) => {
  const selectId = useId();
  const { setSelected, selected } = useSelectContext();

  const { trigger, base } = selectVariants();

  const SelectedItem = Children.toArray(children).find(
    (e) => isValidElement(e) && selected === (e as ReactElement).props.value,
  ) as { props: { children: ReactElement } };

  return (
    <div className={twMerge(base({ className: classNames?.base }), "relative md:hidden block")}>
      <select
        id={selectId}
        className="absolute top-[-20px] right-0 opacity-0 h-full w-full"
        onChange={(e) => setSelected(e.target.value)}
      >
        {Children.toArray(children).map((child) => {
          if (isValidElement(child)) {
            const { value } = child.props as SelectItemProps;
            return (
              <option key={value} value={value}>
                {typeof child.props.children === "string" ? child.props.children : value}
              </option>
            );
          }
          return null;
        })}
      </select>
      <label htmlFor={selectId} className={trigger({ className: classNames?.trigger })}>
        <span>{SelectedItem?.props.children}</span>
        <IconChevronDown
          className={twMerge("min-w-[20px] min-h-[20px] transition-all duration-300")}
        />
      </label>
    </div>
  );
};

const selectVariants = tv({
  slots: {
    base: "group inline-flex flex-col relative w-fit min-w-[9rem] transition-all  duration-500 leading-none",
    listboxWrapper:
      "rounded-md overflow-hidden max-h-[12rem] w-full transition-all z-50 shadow-account-card top-[3.375rem] bg-rice-25 absolute",
    trigger:
      "w-full inline-flex tap-highlight-transparent flex-row items-center justify-between px-4 py-3 gap-3 outline-none shadow-account-card diatype-m-regular h-[46px] rounded-md bg-rice-25",
    icon: "top-[10px] right-4 absolute pointer-events-none min-w-[20px] min-h-[20px] transition-all duration-300",
  },
  variants: {
    isDisabled: {
      true: {
        trigger: "bg-gray-200 text-gray-400 cursor-not-allowed",
      },
    },
  },
});

const ExportComponent = Object.assign(Root, {
  Item,
});

export { ExportComponent as Select };
