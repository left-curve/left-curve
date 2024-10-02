import {
  type ForwardRefRenderFunction,
  type Ref,
  type RefObject,
  forwardRef as baseForwardRef,
  useImperativeHandle,
  useRef,
} from "react";

import type { As, InternalForwardRefRenderFunction, PropsOf, RightJoinProps } from "~/types/react";

export function useDOMRef<T extends HTMLElement = HTMLElement>(
  ref?: RefObject<T | null> | Ref<T | null>,
) {
  const domRef = useRef<T>(null);

  useImperativeHandle(ref, () => domRef.current);

  return domRef;
}

export function forwardRef<
  Component extends As,
  Props extends object,
  OmitKeys extends keyof any = never,
>(
  component: ForwardRefRenderFunction<
    any,
    RightJoinProps<PropsOf<Component>, Props> & {
      as?: As;
    }
  >,
) {
  return baseForwardRef(component as any) as InternalForwardRefRenderFunction<
    Component,
    Props,
    OmitKeys
  >;
}
