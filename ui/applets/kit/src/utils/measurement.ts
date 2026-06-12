// Round sub-pixel measurements so ResizeObserver jitter does not retrigger render loops.
export const roundMeasuredLayoutValue = (value: number) => Math.round(value * 100) / 100;
