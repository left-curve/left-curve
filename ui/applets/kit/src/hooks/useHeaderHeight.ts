import { useLayoutEffect, useState } from "react";

export function useHeaderHeight() {
  const [height, setHeight] = useState(0);

  useLayoutEffect(() => {
    const header = document.querySelector("header");
    if (!header) return;

    const updateHeight = () => setHeight(header.getBoundingClientRect().height);
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
