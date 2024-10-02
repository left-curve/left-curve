import * as LabelPrimitive from "@radix-ui/react-label";
import * as React from "react";

import { type VariantProps, tv } from "tailwind-variants";

const labelVariants = tv(
  {
    base: "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
  },
  {
    twMerge: true,
  },
);

const Label = React.forwardRef<
  React.ElementRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root> & VariantProps<typeof labelVariants>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root ref={ref} className={labelVariants({ className })} {...props} />
));

Label.displayName = "Label";

export { Label };