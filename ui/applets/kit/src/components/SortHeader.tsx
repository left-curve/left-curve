import { IconChevronDownFill, IconChevronUpDown, IconChevronUpFill } from "@left-curve/applets-kit";
import type { Dir } from "../hooks/useTableSort";

export type SortHeaderProps<K extends string> = {
  label: string;
  sortField: K;
  sortKey: K;
  sortDir?: Dir;
  onClick: (key: K) => void;
  className?: string;
};

export const SortHeader: React.FC<SortHeaderProps<string>> = ({
  sortField,
  label,
  sortKey,
  sortDir,
  onClick,
  className,
}) => {
  const active = sortKey === sortField;

  return (
    <button
      type="button"
      onClick={() => onClick(sortField)}
      className={["flex items-center gap-1", className].filter(Boolean).join(" ")}
    >
      <span>{label}</span>
      {!active && <IconChevronUpDown className="w-3 h-3 text-ink-tertiary-500" />}
      {active && sortDir === "asc" && (
        <IconChevronUpFill className="w-3 h-3 text-ink-tertiary-500" />
      )}
      {active && sortDir === "desc" && (
        <IconChevronDownFill className="w-3 h-3 text-ink-tertiary-500" />
      )}
    </button>
  );
};
