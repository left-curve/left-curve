export const numberMask = (v: string, prev: string) => {
  const regex = /^\d+(\.\d{0,18})?$/;
  if (v === "" || regex.test(v)) return v;
  return prev;
};

export const ethAddressMask = (v: string, prev: string) => {
  if (v === "") return v;
  if (v.length > 42) return prev;
  const regex = /^0(x[0-9a-fA-F]*)?$/;

  if (regex.test(v)) return v;

  return prev;
};
