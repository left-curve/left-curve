export type As<Props = any> = React.ElementType<Props>;

export type PropsOf<T extends As> = React.ComponentPropsWithoutRef<T> & {
  as?: As;
};

export type RightJoinProps<
  SourceProps extends object = object,
  OverrideProps extends object = object,
> = OmitCommonProps<SourceProps, keyof OverrideProps> & OverrideProps;

export type OmitCommonProps<Target, OmitAdditionalProps extends keyof any = never> = Omit<
  Target,
  "transition" | "as" | "color" | OmitAdditionalProps
>;

export type MergeWithAs<
  ComponentProps extends object,
  AsProps extends object,
  AdditionalProps extends object = object,
  AsComponent extends As = As,
> = (RightJoinProps<ComponentProps, AdditionalProps> | RightJoinProps<AsProps, AdditionalProps>) & {
  as?: AsComponent;
};

export type InternalForwardRefRenderFunction<
  Component extends As,
  Props extends object = object,
  OmitKeys extends keyof any = never,
> = {
  <AsComponent extends As = Component>(
    props: MergeWithAs<
      React.ComponentPropsWithRef<Component>,
      Omit<React.ComponentPropsWithRef<AsComponent>, OmitKeys>,
      Props,
      AsComponent
    >,
  ): React.ReactElement | null;
  readonly $$typeof: symbol;
  defaultProps?: Partial<Props> | undefined;
  propTypes?: React.WeakValidationMap<Props> | undefined;
  displayName?: string | undefined;
};
