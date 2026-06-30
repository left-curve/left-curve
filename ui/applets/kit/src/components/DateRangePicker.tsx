import { useState } from "react";
import { DayPicker } from "react-day-picker";
import { Dialog, DialogBackdrop, DialogPanel } from "@headlessui/react";
import { formatDate, twMerge } from "@left-curve/foundation";
import { tv } from "tailwind-variants";

import { useMediaQuery } from "../hooks/useMediaQuery";
import { Popover } from "./Popover";
import { IconCalendar } from "./icons/IconCalendar";
import { IconChevronLeft } from "./icons/IconChevronLeft";
import { IconChevronRight } from "./icons/IconChevronRight";

import type { DateRange } from "react-day-picker";

export type DateRangePickerValue = {
  from: Date | undefined;
  to: Date | undefined;
};

export type DateRangePickerProps = {
  value: DateRangePickerValue;
  onChange: (value: DateRangePickerValue) => void;
  numberOfMonths?: number;
  disabled?: (date: Date) => boolean;
  className?: string;
  triggerClassName?: string;
  placeholder?: string;
};

const formatTriggerLabel = (range: DateRangePickerValue, placeholder: string): string => {
  if (!range.from && !range.to) return placeholder;
  if (range.from && !range.to) return formatDate(range.from, "MMM do, yyyy");
  if (range.from && range.to) {
    return `${formatDate(range.from, "MMM do, yyyy")} → ${formatDate(range.to, "MMM do, yyyy")}`;
  }
  return placeholder;
};

const toRange = (value: DateRangePickerValue): DateRange | undefined =>
  value.from ? { from: value.from, to: value.to } : undefined;

export function DateRangePicker({
  value,
  onChange,
  numberOfMonths = 2,
  disabled,
  className,
  triggerClassName,
  placeholder = "Select range",
}: DateRangePickerProps) {
  const { isMd } = useMediaQuery();
  const [pendingRange, setPendingRange] = useState<DateRange | undefined>(() => toRange(value));
  const [mobileOpen, setMobileOpen] = useState(false);

  const valueKey = `${value.from?.getTime() ?? ""}-${value.to?.getTime() ?? ""}`;
  const [lastValueKey, setLastValueKey] = useState(valueKey);
  if (valueKey !== lastValueKey) {
    setLastValueKey(valueKey);
    setPendingRange(toRange(value));
  }

  const handleSelect = (range: DateRange | undefined, triggerDate: Date) => {
    const hadCompleteRange = !!(pendingRange?.from && pendingRange?.to);

    if (hadCompleteRange) {
      setPendingRange({ from: triggerDate, to: undefined });
      return;
    }

    setPendingRange(range);
    if (range?.from && range?.to) {
      onChange({ from: range.from, to: range.to });
    }
  };

  const triggerLabel = formatTriggerLabel(value, placeholder);
  const triggerContent = (
    <>
      <span className="whitespace-nowrap">{triggerLabel}</span>
      <IconCalendar className="w-3.5 h-3.5 shrink-0" />
    </>
  );

  const calendar = (effectiveNumberOfMonths: number) => (
    <DayPicker
      mode="range"
      selected={pendingRange}
      onSelect={handleSelect}
      numberOfMonths={effectiveNumberOfMonths}
      fixedWeeks
      showOutsideDays
      disabled={disabled}
      components={{
        Nav: ({ onPreviousClick, onNextClick, previousMonth, nextMonth }) => (
          <nav className="absolute inset-x-0 top-0 flex items-center justify-between z-10 pointer-events-none">
            <button
              type="button"
              onClick={onPreviousClick}
              disabled={!previousMonth}
              aria-label="Previous month"
              className="pointer-events-auto p-1 text-ink-secondary-700 hover:text-ink-primary-900 disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer transition-colors"
            >
              <IconChevronLeft className="w-6 h-6" />
            </button>
            <button
              type="button"
              onClick={onNextClick}
              disabled={!nextMonth}
              aria-label="Next month"
              className="pointer-events-auto p-1 text-ink-secondary-700 hover:text-ink-primary-900 disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer transition-colors"
            >
              <IconChevronRight className="w-6 h-6" />
            </button>
          </nav>
        ),
        DayButton: ({ day, modifiers, className: _dayClassName, ...props }) => {
          const isOutside = !!modifiers.outside;

          return (
            <button
              type="button"
              {...props}
              className={dayButton({
                isStart: !isOutside && !!modifiers.range_start,
                isEnd: !isOutside && !!modifiers.range_end,
                isMiddle: !isOutside && !!modifiers.range_middle,
                isSelected: !isOutside && !!modifiers.selected,
                isOutside,
              })}
            >
              {day.date.getDate()}
            </button>
          );
        },
      }}
      formatters={{
        formatWeekdayName: (date) => formatDate(date, "EEEEEE"),
        formatCaption: (date) => formatDate(date, "MMMM yyyy"),
      }}
      classNames={dayPickerClassNames}
    />
  );

  if (isMd) {
    return (
      <Popover
        showArrow={false}
        classNames={{
          base: className,
          trigger: trigger({ className: triggerClassName }),
          menu: "p-6 bg-surface-secondary-rice rounded-xl shadow-account-card",
        }}
        trigger={triggerContent}
        menu={calendar(numberOfMonths)}
      />
    );
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setMobileOpen(true)}
        className={trigger({ asButton: true, className: twMerge(triggerClassName, className) })}
      >
        {triggerContent}
      </button>

      <Dialog
        open={mobileOpen}
        onClose={setMobileOpen}
        transition
        className="relative z-50 transition duration-200 data-[closed]:opacity-0"
      >
        <DialogBackdrop className="fixed inset-0 bg-primitives-gray-light-900/50" />
        <div className="fixed inset-x-0 bottom-0 flex justify-center">
          <DialogPanel
            transition
            className="w-full bg-surface-secondary-rice rounded-t-xl shadow-account-card p-6 pb-[calc(env(safe-area-inset-bottom,0px)+2.5rem)] max-h-[90vh] overflow-y-auto transition duration-250 ease-out data-[closed]:translate-y-full"
          >
            <div className="mx-auto mb-4 h-1 w-10 rounded-full bg-outline-secondary-gray" />
            {calendar(1)}
          </DialogPanel>
        </div>
      </Dialog>
    </>
  );
}

const trigger = tv({
  base: "exposure-xs-italic text-ink-secondary-blue hover:text-primitives-blue-light-600 transition-colors gap-1.5 whitespace-nowrap",
  variants: {
    asButton: { true: "flex items-center" },
  },
});

const dayButton = tv(
  {
    base: "w-full h-full flex items-center justify-center cursor-pointer transition-colors diatype-sm-medium text-ink-secondary-700 outline-none focus:outline-none focus-visible:outline-none rounded-md",
    variants: {
      isStart: { true: "" },
      isEnd: { true: "" },
      isMiddle: { true: "" },
      isSelected: { true: "" },
      isOutside: { true: "" },
    },
    compoundVariants: [
      {
        isStart: false,
        isEnd: false,
        isMiddle: false,
        isSelected: false,
        isOutside: false,
        class: "hover:bg-surface-tertiary-rice",
      },
      {
        isStart: false,
        isEnd: false,
        isMiddle: false,
        isSelected: false,
        isOutside: true,
        class: "text-ink-tertiary-500 opacity-40 hover:bg-surface-tertiary-rice",
      },
      {
        isStart: true,
        isEnd: true,
        class: "bg-brand-red-bean text-white shadow-btn-shadow-gradient rounded-lg",
      },
      {
        isStart: true,
        isEnd: false,
        class:
          "bg-brand-red-bean text-white shadow-btn-shadow-gradient rounded-l-lg rounded-r-none relative after:content-[''] after:absolute after:left-full after:top-0 after:bottom-0 after:w-[2px] after:bg-surface-primary-red",
      },
      {
        isStart: false,
        isEnd: true,
        class:
          "bg-brand-red-bean text-white shadow-btn-shadow-gradient rounded-r-lg rounded-l-none",
      },
      {
        isStart: false,
        isEnd: false,
        isMiddle: true,
        class:
          "bg-surface-primary-red text-ink-secondary-700 rounded-none hover:bg-surface-secondary-red relative after:content-[''] after:absolute after:left-full after:top-0 after:bottom-0 after:w-[2px] after:bg-surface-primary-red",
      },
      {
        isStart: false,
        isEnd: false,
        isMiddle: false,
        isSelected: true,
        class: "bg-brand-red-bean text-white shadow-btn-shadow-gradient rounded-lg",
      },
    ],
  },
  { twMerge: true },
);

const dayPickerClassNames = {
  root: "rdp text-ink-primary-900 relative w-fit mx-auto",
  months: "flex justify-center",
  month:
    "flex flex-col gap-2 md:px-6 md:first:pl-0 md:last:pr-0 md:border-r md:border-outline-secondary-gray md:last:border-r-0 relative",
  month_caption:
    "flex justify-center items-center h-8 exposure-sm-italic text-ink-primary-900 mb-2",
  weekdays: "grid grid-cols-7",
  weekday:
    "h-9 w-9 flex items-center justify-center diatype-xs-medium text-ink-placeholder-400 uppercase font-normal",
  week: "grid grid-cols-7 [&:nth-of-type(6)]:hidden",
  day: "h-9 w-9 text-ink-secondary-700",
  month_grid: "w-fit border-separate border-spacing-0",
  today: "[&>button]:underline [&>button]:underline-offset-4",
  outside: "",
  disabled: "opacity-30 cursor-not-allowed",
  hidden: "invisible",
};
