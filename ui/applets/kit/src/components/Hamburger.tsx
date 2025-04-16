import { twMerge } from "#utils/twMerge.js";
import { IconButton } from "./IconButton";

interface Props {
  isOpen: boolean;
  className?: string;
  onClick: () => void;
}

export const Hamburger: React.FC<Props> = ({ isOpen, className, onClick }) => {
  return (
    <IconButton
      variant="utility"
      size="lg"
      className={twMerge("relative group", className)}
      type="button"
      onClick={onClick}
    >
      <div className="relative flex overflow-hidden items-center justify-center transform transition-all duration-200">
        <div
          className={twMerge(
            "flex flex-col justify-between transform transition-all duration-200 origin-center overflow-hidden",
            isOpen ? "gap-2" : "gap-1",
          )}
        >
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 origin-left",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 delay-75",
              { "translate-x-10": isOpen },
            )}
          />
          <div
            className={twMerge(
              "bg-rice-700 h-[2px] w-4 rounded-xl transform transition-all duration-200 origin-left delay-150",
              { "translate-x-10": isOpen },
            )}
          />

          <div
            className={twMerge(
              "absolute items-center justify-between transform transition-all duration-300 top-2.5 -translate-x-10 flex w-0",
              { "translate-x-[-1.5px] translate-y-[1px] w-12": isOpen },
            )}
          >
            <div
              className={twMerge(
                "absolute bg-rice-700 h-[2px] w-5 rounded-full transform transition-all duration-300 rotate-0 delay-200",
                { "rotate-45": isOpen },
              )}
            />
            <div
              className={twMerge(
                "absolute bg-rice-700 h-[2px] w-5 rounded-full transform transition-all duration-300 -rotate-0 delay-200",
                { "-rotate-45": isOpen },
              )}
            />
          </div>
        </div>
      </div>
    </IconButton>
  );
};
