import { useLayoutEffect, useState } from "react";
import { roundMeasuredLayoutValue } from "../utils/measurement.js";

export function useHeaderHeight() {
  const [height, setHeight] = useState(0);

  useLayoutEffect(() => {
    const header = document.querySelector("header");
    if (!header) return;

    const updateHeight = () => {
      const nextHeight = roundMeasuredLayoutValue(header.getBoundingClientRect().height);
      setHeight(nextHeight);
    };
    updateHeight();

    const observer = new ResizeObserver(updateHeight);
    observer.observe(header);

    window.addEventListener("resize", updateHeight);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", updateHeight);
    };
  }, []);

  return height;
}
