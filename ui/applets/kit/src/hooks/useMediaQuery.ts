import { useEffect, useState } from "react";

const medias = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
  "3xl": 2272,
};

type MediaQueries = {
  isSm: boolean;
  isMd: boolean;
  isLg: boolean;
  isXl: boolean;
  is2Xl: boolean;
  is3Xl: boolean;
};

export const useMediaQuery = () => {
  const [matchSize, setMatchSize] = useState<MediaQueries>({
    isSm: window.innerWidth >= medias.sm,
    isMd: window.innerWidth >= medias.md,
    isLg: window.innerWidth >= medias.lg,
    isXl: window.innerWidth >= medias.xl,
    is2Xl: window.innerWidth >= medias["2xl"],
    is3Xl: window.innerWidth >= medias["3xl"],
  });

  useEffect(() => {
    const handleResize = () => {
      setMatchSize({
        isSm: window.innerWidth >= medias.sm,
        isMd: window.innerWidth >= medias.md,
        isLg: window.innerWidth >= medias.lg,
        isXl: window.innerWidth >= medias.xl,
        is2Xl: window.innerWidth >= medias["2xl"],
        is3Xl: window.innerWidth >= medias["3xl"],
      });
    };
    window.addEventListener("resize", handleResize);
    handleResize();
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  return matchSize;
};
