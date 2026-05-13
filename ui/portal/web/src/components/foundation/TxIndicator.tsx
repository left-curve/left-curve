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
  const { subscriptions, toast } = useApp();
  const [isSubmittingTx, setIsSubmittingTx] = useState(false);
  const [indicator, setIndicator] = useState<keyof typeof Indicators>("spinner");

  const IndicatorComponent = Indicators[indicator];

  useEffect(() => {
    const unsubscribe = subscriptions.subscribe("submitTx", {
      listener: (event) => {
        switch (event.status) {
          case "pending":
            setIndicator("spinner");
            setIsSubmittingTx(true);
            break;
          case "success":
            setIndicator("success");
            setTimeout(() => setIsSubmittingTx(false), 1500);
            break;
          case "error":
            setIndicator("error");
            toast.error({ title: event.title, description: event.description });
            setTimeout(() => setIsSubmittingTx(false), 1500);
            break;
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
        "text-primitives-green-light-300 w-6 h-6": indicator === "success",
        "text-primitives-red-light-300 w-6 h-6": indicator === "error",
      })}
    />
  ) : (
    icon
  );
};
