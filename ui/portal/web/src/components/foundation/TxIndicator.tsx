import {
  CheckCircleIcon,
  IconButton,
  Spinner,
  XCircleIcon,
  twMerge,
} from "@left-curve/applets-kit";
import type React from "react";
import { type PropsWithChildren, useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

const Indicators = {
  spinner: Spinner,
  success: CheckCircleIcon,
  error: XCircleIcon,
};

export const TxIndicator: React.FC<PropsWithChildren> = ({ children }) => {
  const { eventBus } = useApp();
  const [isSubmittingTx, setIsSubmittingTx] = useState(false);
  const [indicator, setIndicator] = useState<keyof typeof Indicators>("spinner");

  const IndicatorComponent = Indicators[indicator];

  useEffect(() => {
    const unsubscribe = eventBus.subscribe("submit_tx", ({ isSubmitting, txResult }) => {
      if (isSubmitting) {
        setIndicator("spinner");
        setIsSubmittingTx(isSubmitting);
      } else {
        setIndicator(txResult.hasSucceeded ? "success" : "error");
        setTimeout(() => {
          setIsSubmittingTx(false);
        }, 1500);
      }
    });
    return () => unsubscribe();
  }, []);

  return isSubmittingTx ? (
    <IconButton variant="utility" size="lg" type="button">
      <IndicatorComponent
        size="sm"
        color="current"
        className={twMerge({
          "stroke-2 stroke-status-success w-6 h-6": indicator === "success",
          "stroke-2 stroke-red-bean-400 w-6 h-6": indicator === "error",
        })}
      />
    </IconButton>
  ) : (
    <>{children}</>
  );
};
