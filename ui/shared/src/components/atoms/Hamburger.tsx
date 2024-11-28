import type { ComponentPropsWithoutRef } from "react";
import { twMerge } from "../../utils";

interface Props {
  isOpen: boolean;
}

export const Hamburger: React.FC<ComponentPropsWithoutRef<"button"> & Props> = ({
  isOpen,
  className,
  ...props
}) => {
  return (
    <button
      className={twMerge("relative group", className)}
      hamburger-element="true"
      type="button"
      {...props}
    >
      <div
        hamburger-element="true"
        className="relative flex overflow-hidden items-center justify-center transform transition-allduration-200"
      >
        <div
          hamburger-element="true"
          className={twMerge(
            "flex flex-col justify-between transform transition-all duration-200 origin-center overflow-hidden",
            isOpen ? "gap-2" : "gap-1",
          )}
        >
          <div
            className={twMerge(
              "bg-typography-black-300 h-[2px] w-4 rounded-3xl transform transition-all duration-200 origin-left",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-typography-black-300 h-[2px] w-4 rounded-3xl transform transition-all duration-200 delay-75",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-typography-black-300 h-[2px] w-4 rounded-3xl transform transition-all duration-200 origin-left delay-150",
              { "translate-x-10": isOpen },
            )}
          />

          <div
            hamburger-element="true"
            className={twMerge(
              "absolute items-center justify-between transform transition-all duration-300 top-2.5 -translate-x-10 flex w-0",
              { "translate-x-[-1.5px] translate-y-[1px] w-12": isOpen },
            )}
          >
            <div
              hamburger-element="true"
              className={twMerge(
                "absolute bg-typography-black-300 h-[2px] w-5 rounded-full transform transition-all duration-300 rotate-0 delay-200",
                { "rotate-45": isOpen },
              )}
            />
            <div
              hamburger-element="true"
              className={twMerge(
                "absolute bg-typography-black-300 h-[2px] w-5 rounded-full transform transition-all duration-300 -rotate-0 delay-200",
                { "-rotate-45": isOpen },
              )}
            />
          </div>
        </div>
      </div>
    </button>
  );
};
