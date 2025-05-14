export const numberMask = (v: string, prev: string) => {
  const regex = /^\d+(\.\d{0,18})?$/;
  if (v === "" || regex.test(v)) return v;
  return prev;
};
