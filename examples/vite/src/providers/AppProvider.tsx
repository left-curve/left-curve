import { ModalRoot } from "@leftcurve/react/components";
import type { PropsWithChildren } from "react";
import { GrugProvider } from "./GrugProvider";

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <GrugProvider>
      {children}
      <ModalRoot />
    </GrugProvider>
  );
};
