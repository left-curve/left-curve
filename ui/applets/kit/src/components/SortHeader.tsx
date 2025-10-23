import {
  IconChevronDownFill,
  IconChevronUpDown,
  IconChevronUpFill,
  twMerge,
} from "@left-curve/applets-kit";

export type SortHeaderProps = {
  label: string;
  sorted: false | "asc" | "desc";
  toggleSort: (desc?: boolean) => void;
  className?: string;
};

export const SortHeader: React.FC<SortHeaderProps> = ({ label, sorted, toggleSort, className }) => {
  const isActive = sorted !== false;

  return (
    <button
      type="button"
      onClick={() => toggleSort(sorted === "asc")}
      className={twMerge("flex items-center gap-1", className)}
    >
      <span>{label}</span>
      {!isActive && <IconChevronUpDown className="w-3 h-3 text-ink-tertiary-500" />}
      {isActive && sorted === "asc" && (
        <IconChevronUpFill className="w-3 h-3 text-ink-tertiary-500" />
      )}
      {isActive && sorted === "desc" && (
        <IconChevronDownFill className="w-3 h-3 text-ink-tertiary-500" />
      )}
    </button>
  );
};
