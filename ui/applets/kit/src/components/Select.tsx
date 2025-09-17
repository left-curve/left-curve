import { Children, isValidElement, useContext, useId, useRef, useState } from "react";

import { useClickAway } from "../hooks/useClickAway.js";
import { createContext, useControlledState } from "@left-curve/foundation";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDownFill } from "./icons/IconChevronDownFill";

import { tv } from "tailwind-variants";
import { twMerge } from "@left-curve/foundation";

import type { PropsWithChildren, ReactElement } from "react";
import type React from "react";
import type { VariantProps } from "tailwind-variants";

const [Provider, useSelect] = createContext<{
  selected: string;
  setSelected: (val: string) => void;
  slots: ReturnType<typeof selectVariants>;
  classNames?: SelectProps["classNames"];
}>({ name: "SelectContext", strict: true });

export interface SelectProps extends VariantProps<typeof selectVariants> {
  defaultValue?: string;
  onChange?: (value: string) => void;
  value?: string;
  variant?: "boxed" | "plain";
  classNames?: {
    base?: string;
    listboxWrapper?: string;
    listBoxContainer?: string;
    listBoxItem?: string;
    listbox?: string;
    value?: string;
    trigger?: string;
    icon?: string;
  };
}

const Root: React.FC<PropsWithChildren<SelectProps>> = (props) => {
  const {
    classNames,
    children,
    onChange,
    value,
    defaultValue,
    isDisabled,
    variant = "boxed",
  } = props;

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

  const slots = selectVariants({ isDisabled, variant });
  const { base, trigger, listboxWrapper, icon, listBoxContainer } = slots;

  useClickAway(selectRef, () => setIsOpen(false));

  return (
    <Provider value={{ selected, setSelected, slots, classNames }}>
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
            <IconChevronDownFill
              className={twMerge(icon(), classNames?.icon, { "rotate-180": isOpen })}
            />
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
                    className={twMerge(listBoxContainer(), classNames?.listBoxContainer)}
                  >
                    {children}
                  </motion.ul>
                </motion.div>
              )}
            </AnimatePresence>
          </motion.div>
        </div>
      </div>
    </Provider>
  );
};

type SelectItemProps = {
  value: string;
  className?: string;
};

const Item: React.FC<PropsWithChildren<SelectItemProps>> = ({ value, children }) => {
  const { setSelected, slots, classNames } = useSelect();

  return (
    <li
      value={value}
      onClick={() => setSelected(value)}
      className={twMerge(slots.listBoxItem(), classNames?.listBoxItem)}
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
  const { setSelected, selected } = useSelect();

  const { trigger, base } = selectVariants();

  const SelectedItem = Children.toArray(children).find(
    (e) => isValidElement(e) && selected === (e as ReactElement).props.value,
  ) as { props: { children: ReactElement } };

  return (
    <div className={twMerge("relative md:hidden block", base({ className: classNames?.base }))}>
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
        <IconChevronDownFill className={twMerge("w-4 h-4 transition-all duration-300")} />
      </label>
    </div>
  );
};

const selectVariants = tv({
  slots: {
    base: "group inline-flex flex-col relative w-fit transition-all duration-500 leading-none",
    listboxWrapper:
      "overflow-hidden max-h-[12rem] transition-all z-50 shadow-account-card bg-surface-secondary-rice absolute min-w-full w-max",
    listBoxContainer:
      "max-h-[12rem] outline-none gap-1 flex flex-col scrollbar-none overflow-auto min-w-full w-max",
    listBoxItem: "outline-none cursor-pointer flex items-center transition-all leading-none",
    trigger:
      "w-full inline-flex tap-highlight-transparent flex-row items-center justify-between outline-none",
    icon: "pointer-events-none w-4 h-4 transition-all duration-300",
  },
  variants: {
    variant: {
      boxed: {
        base: "",
        listboxWrapper: "rounded-md top-[calc(100%+0.5rem)]",
        listBoxContainer: "p-2",
        listBoxItem:
          "rounded-sm py-2 px-3 text-base diatype-m-medium bg-surface-secondary-rice hover:bg-surface-tertiary-rice",
        trigger:
          "shadow-account-card bg-surface-secondary-rice h-[46px] px-4 py-3 rounded-md diatype-m-regular gap-3",
      },
      plain: {
        base: "min-w-fit",
        listboxWrapper: "diatype-xs-regular rounded-sm top-[calc(100%+0.2rem)]",
        listBoxContainer: "p-1",
        listBoxItem:
          "rounded-sm py-1 px-2 text-sm diatype-xs-regular bg-surface-secondary-rice hover:bg-surface-tertiary-rice",
        trigger: "diatype-xs-regular group-hover:text-primary-900 gap-1",
        icon: "w-3 h-3 group-hover:text-primary-900",
      },
    },
    isDisabled: {
      true: {
        trigger: "bg-secondary-gray text-tertiary-500 cursor-not-allowed",
      },
    },
  },
});

export const Select = Object.assign(Root, {
  Item,
});
