type withAsProp<T> = { as?: T };

type PolymorphicComponentProps<T extends React.ElementType, P = object> = React.PropsWithChildren<
  P & withAsProp<T>
> &
  Omit<React.ComponentPropsWithoutRef<T>, keyof (P & withAsProp<T>)>;

export type PolymorphicRenderFunction<T extends React.ElementType, P = object> = (
  props: React.PropsWithoutRef<PolymorphicComponentProps<T, P>>,
  ref: React.ForwardedRef<React.ElementRef<T>>,
) => React.ReactElement | null;

export type PolymorphicComponent<T extends React.ElementType, P> = <
  Element extends React.ElementType = T,
>(
  props: PolymorphicComponentProps<Element, P> & {
    ref?: React.ForwardedRef<React.ElementRef<Element>>;
  },
) => React.ReactElement;
