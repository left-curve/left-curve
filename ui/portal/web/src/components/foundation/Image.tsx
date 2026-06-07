import { forwardRef, type ComponentPropsWithoutRef } from "react";

export type ImageProps = ComponentPropsWithoutRef<"img">;

export const Image = forwardRef<HTMLImageElement, ImageProps>(({ src, alt, ...props }, ref) => (
  <img ref={ref} src={src} alt={alt} {...props} />
));

Image.displayName = "Image";
