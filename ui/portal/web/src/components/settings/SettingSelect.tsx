import { useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";
import { twMerge } from "@left-curve/foundation";
import { IconChevronRight, Select, useMediaQuery, type SelectRef } from "@left-curve/applets-kit";

import type { ReactNode } from "react";

export interface SettingSelectOption {
  value: string;
  label: ReactNode;
}

export interface SettingSelectProps {
  value: string;
  onChange: (value: string) => void;
  options: SettingSelectOption[];
  icon: ReactNode;
  label: string;
}

export const SettingSelect: React.FC<SettingSelectProps> = ({
  value,
  onChange,
  options,
  icon,
  label,
}) => {
  const { isMd } = useMediaQuery();
  const [isOpen, setIsOpen] = useState(false);
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value);

  const handleSelect = (optionValue: string) => {
    onChange(optionValue);
    setIsOpen(false);
  };

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          {icon}
          <span className="diatype-m-bold text-ink-secondary-700">{label}</span>
        </p>
        <Select ref={selectRef} containerRef={containerRef} value={value} onChange={onChange}>
          {options.map((option) => (
            <Select.Item key={option.value} value={option.value}>
              {option.label}
            </Select.Item>
          ))}
        </Select>
      </div>
    );
  }

  return (
    <>
      <div
        className="flex items-center justify-between px-2 py-4 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => setIsOpen(true)}
      >
        <p className="flex items-center justify-center gap-2">
          {icon}
          <span className="diatype-m-bold text-ink-secondary-700">{label}</span>
        </p>
        <div className="flex items-center gap-1">
          <span className="diatype-m-regular text-ink-tertiary-500">{selectedOption?.label}</span>
          <IconChevronRight className="w-4 h-4 text-ink-tertiary-500" />
        </div>
      </div>
      <Sheet
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        detent="content-height"
        disableScrollLocking
      >
        <Sheet.Container className="!bg-surface-primary-rice !rounded-t-2xl !shadow-none">
          <Sheet.Header />
          <Sheet.Content>
            <div className="px-4 pb-8">
              <h3 className="diatype-lg-bold text-ink-primary-900 mb-4">{label}</h3>
              <div className="flex flex-col gap-2">
                {options.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    onClick={() => handleSelect(option.value)}
                    className={twMerge(
                      "w-full text-left px-4 py-3 rounded-lg diatype-m-medium transition-all",
                      option.value === value
                        ? "bg-surface-tertiary-rice text-ink-primary-900"
                        : "hover:bg-surface-tertiary-rice text-ink-secondary-700",
                    )}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          </Sheet.Content>
        </Sheet.Container>
        <Sheet.Backdrop onTap={() => setIsOpen(false)} />
      </Sheet>
    </>
  );
};
