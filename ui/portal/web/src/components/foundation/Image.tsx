import { forwardRef, type ComponentPropsWithoutRef } from "react";

import { imageUrl } from "~/asset-url";

export type ImageProps = ComponentPropsWithoutRef<"img">;

export const Image = forwardRef<HTMLImageElement, ImageProps>(({ src, alt, ...props }, ref) => (
  <img ref={ref} src={typeof src === "string" ? imageUrl(src) : src} alt={alt} {...props} />
));

Image.displayName = "Image";
