import { createContext } from "@left-curve/applets-kit";
import type React from "react";
import { type PropsWithChildren, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ChestOpeningOverlay, prefetchChestImages } from "./ChestOpeningOverlay";

type BoxVariant = "bronze" | "silver" | "gold" | "crystal";

type ChestOpeningContextValue = {
  openChest: (variant: BoxVariant) => void;
  closeChest: () => void;
  isOpen: boolean;
  currentVariant: BoxVariant | null;
};

const [ChestOpeningContextProvider, useChestOpeningContext] = createContext<ChestOpeningContextValue>({
  name: "ChestOpeningContext",
});

export const ChestOpeningProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const [currentVariant, setCurrentVariant] = useState<BoxVariant | null>(null);

  // Prefetch images when provider mounts (early in app lifecycle)
  useEffect(() => {
    prefetchChestImages();
  }, []);

  const openChest = (variant: BoxVariant) => {
    setCurrentVariant(variant);
  };

  const closeChest = () => {
    setCurrentVariant(null);
  };

  const isOpen = currentVariant !== null;

  return (
    <ChestOpeningContextProvider value={{ openChest, closeChest, isOpen, currentVariant }}>
      {children}
      {isOpen &&
        createPortal(
          <ChestOpeningOverlay variant={currentVariant!} onClose={closeChest} />,
          document.body,
        )}
    </ChestOpeningContextProvider>
  );
};

export const useChestOpening = useChestOpeningContext;
