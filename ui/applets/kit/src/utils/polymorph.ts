import { forwardRef } from "react";

import type { ElementType } from "react";
import type { PolymorphicComponent, PolymorphicRenderFunction } from "#types/polymorph.js";

export const forwardRefPolymorphic = <T extends ElementType = "div", P = object>(
  render: PolymorphicRenderFunction<T, P>,
  displayName?: string,
) => {
  const Component = forwardRef(render);
  Component.displayName = displayName || "PolymorphicComponent";
  return Component as unknown as PolymorphicComponent<T, P>;
};
