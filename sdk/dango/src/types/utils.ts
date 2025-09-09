export type WithPrice<T = object, Price = number> = T & {
  price: Price;
};

export type WithAmount<T = object, Amount = string> = T & {
  amount: Amount;
};

export type WithDecimals<T = object, Decimals = number> = T & {
  decimals: Decimals;
};
