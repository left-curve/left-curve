import type {
  ComponentPropsWithoutRef,
  DetailedHTMLProps,
  ElementType,
  ForwardedRef,
  HTMLAttributes,
  PropsWithChildren,
  PropsWithoutRef,
  ReactElement,
} from "react";

export type ElementTypeToHTMLElement<T extends ElementType> = T extends keyof JSX.IntrinsicElements
  ? JSX.IntrinsicElements[T] extends DetailedHTMLProps<HTMLAttributes<infer E>, any>
    ? E
    : never
  : never;

type withAsProp<T> = { as?: T };

type PolymorphicComponentProps<T extends ElementType, P = object> = PropsWithChildren<
  P & withAsProp<T>
> &
  Omit<ComponentPropsWithoutRef<T>, keyof (P & withAsProp<T>)>;

export type PolymorphicRenderFunction<T extends ElementType, P = object> = (
  props: PropsWithoutRef<PolymorphicComponentProps<T, P>>,
  ref: ForwardedRef<ElementTypeToHTMLElement<T>>,
) => ReactElement | null;

export type PolymorphicComponent<T extends ElementType, P> = <Element extends ElementType = T>(
  props: PolymorphicComponentProps<Element, P> & {
    ref?: ForwardedRef<ElementTypeToHTMLElement<Element>>;
  },
) => ReactElement;
