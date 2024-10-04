import { useEffect, useState } from "react";

type MediaBreakpoints = "sm" | "md" | "lg" | "xl" | "2xl";

const medias = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
};

export const useMediaQuery = (size: MediaBreakpoints) => {
  const [matchSize, setMatchSize] = useState<boolean>(false);

  useEffect(() => {
    const handleResize = () => {
      setMatchSize(() => window.innerWidth >= medias[size]);
    };
    window.addEventListener("resize", handleResize);
    handleResize();
    return () => window.removeEventListener("resize", handleResize);
  }, [size]);

  return matchSize;
};
