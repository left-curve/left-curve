import { useControlledState } from "@left-curve/foundation";

import { Field, Checkbox as HCheckBox, Label } from "@headlessui/react";
import { tv } from "tailwind-variants";

import { twMerge } from "@left-curve/foundation";

import type { VariantProps } from "tailwind-variants";

export interface CheckboxProps extends VariantProps<typeof checkBoxVariants> {
  label?: string;
  className?: string;
  checked?: boolean;
  defaultChecked?: boolean;
  onChange?: (checked: boolean) => void;
}

export const Checkbox: React.FC<CheckboxProps> = ({
  className,
  color,
  size,
  radius,
  isDisabled,
  checked,
  label,
  defaultChecked,
  onChange,
}) => {
  const [inputValue, setInputValue] = useControlledState(checked, onChange, defaultChecked ?? true);

  const styles = checkBoxVariants({
    color,
    size,
    radius,
    isDisabled,
  });

  const labelStyles = labelVariants({
    size,
  });

  return (
    <Field className={twMerge("flex items-center gap-2", className)}>
      <HCheckBox checked={inputValue} onChange={setInputValue} className={twMerge(styles)}>
        <svg
          className="stroke-white opacity-0 group-data-[checked]:opacity-100 transition-all"
          viewBox="0 0 14 14"
          fill="none"
        >
          <path d="M3 8L6 11L11 3.5" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </HCheckBox>
      {label && <Label className={twMerge(labelStyles)}>{label}</Label>}
    </Field>
  );
};

const checkBoxVariants = tv(
  {
    base: "flex items-center justify-center group transition-all border-2 outline-none cursor-pointer",
    variants: {
      color: {
        red: "bg-white data-[checked]:bg-red-bean-400 border-red-bean-400",
        blue: "bg-white data-[checked]:bg-blue-500  border-blue-500",
        grey: "bg-white data-[checked]:bg-gray-500  border-gray-500",
      },
      size: {
        sm: "w-4 h-4",
        md: "w-5 h-5",
        lg: "w-6 h-6",
        xl: "w-7 h-7",
      },
      radius: {
        md: "rounded-[6px]",
        full: "rounded-full",
      },
      isDisabled: {
        true: "pointer-events-none cursor-not-allowed",
      },
    },
    defaultVariants: {
      size: "md",
      color: "red",
      radius: "full",
      isDisabled: false,
    },
  },
  {
    twMerge: true,
  },
);

const labelVariants = tv(
  {
    base: "text-tertiary-500 pt-1 select-none cursor-pointer",
    variants: {
      size: {
        sm: "diatype-sm-regular",
        md: "diatype-m-regular",
        lg: "diatype-lg-regular",
        xl: "h4-regular",
      },
    },
    defaultVariants: {
      size: "md",
      color: "red",
      radius: "full",
      isDisabled: false,
    },
  },
  {
    twMerge: true,
  },
);
