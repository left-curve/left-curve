export type Prettify<T> = {
  [K in keyof T]: T[K];
} & {};

export type OneOf<
  union extends object,
  keys extends KeyofUnion<union> = KeyofUnion<union>,
> = union extends infer Item ? Item & { [K in Exclude<keys, keyof Item>]?: undefined } : never;

type KeyofUnion<type> = type extends type ? keyof type : never;

export type RemoveUndefined<type> = {
  [key in keyof type]: NonNullable<type[key]>;
};

export type ExactPartial<type> = {
  [key in keyof type]?: type[key] | undefined;
};

export type ExactRequired<type> = {
  [P in keyof type]-?: Exclude<type[P], undefined>;
};

/**
 * @description Creates a type that is T with the required keys K.
 *
 * @example
 * RequiredBy<{ a?: string, b: number }, 'a'>
 * => { a: string, b: number }
 */
export type RequiredBy<T, K extends keyof T> = Omit<T, K> & ExactRequired<Pick<T, K>>;

export type StrictOmit<type, keys extends keyof type> = Pick<type, Exclude<keyof type, keys>>;

export type UnionStrictOmit<type, keys extends keyof type> = type extends any
  ? StrictOmit<type, keys>
  : never;

/**
 * @description Creates a type that is T with the optional keys K.
 */
export type OneRequired<T, K1 extends keyof T, K2 extends keyof T> =
  | (Required<Pick<T, K1>> & Partial<Pick<T, K2>>)
  | (Required<Pick<T, K2>> & Partial<Pick<T, K1>>);
