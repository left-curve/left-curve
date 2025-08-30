import clsx, { type ClassValue } from "clsx";
import { cn } from "tailwind-variants";

export function twMerge(...inputs: ClassValue[]) {
  return cn(clsx(inputs))({ twMerge: true });
}
