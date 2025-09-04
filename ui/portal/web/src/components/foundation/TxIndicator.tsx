import {
  IconCheckedCircle,
  IconCloseCircle,
  Spinner,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { useEffect, useState } from "react";

import type React from "react";

type TxIndicatorProps = {
  icon: React.ReactNode;
};

const Indicators = {
  spinner: Spinner,
  success: IconCheckedCircle,
  error: IconCloseCircle,
};

export const TxIndicator: React.FC<TxIndicatorProps> = ({ icon }) => {
  const { subscriptions } = useApp();
  const [isSubmittingTx, setIsSubmittingTx] = useState(false);
  const [indicator, setIndicator] = useState<keyof typeof Indicators>("spinner");

  const IndicatorComponent = Indicators[indicator];

  useEffect(() => {
    const unsubscribe = subscriptions.subscribe("submitTx", {
      listener: ({ isSubmitting, isSuccess }) => {
        if (isSubmitting) {
          setIndicator("spinner");
          setIsSubmittingTx(isSubmitting);
        } else {
          setIndicator(isSuccess ? "success" : "error");
          setTimeout(() => {
            setIsSubmittingTx(false);
          }, 1500);
        }
      },
    });
    return () => unsubscribe();
  }, []);

  return isSubmittingTx ? (
    <IndicatorComponent
      size="md"
      color="current"
      className={twMerge({
        "text-green-bean-300 w-6 h-6": indicator === "success",
        "text-red-bean-300 w-6 h-6": indicator === "error",
      })}
    />
  ) : (
    icon
  );
};
