import { CheckCircleIcon, Spinner, XCircleIcon, twMerge } from "@left-curve/applets-kit";
import { useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

import type React from "react";
import type { PropsWithChildren } from "react";

const Indicators = {
  spinner: Spinner,
  success: CheckCircleIcon,
  error: XCircleIcon,
};

type IndicatorProps<C extends React.ElementType = React.ElementType> = PropsWithChildren<
  {
    as?: C;
  } & React.ComponentPropsWithoutRef<C>
>;

export const TxIndicator = <C extends React.ElementType = React.ElementType>({
  as,
  children,
  ...props
}: IndicatorProps<C>) => {
  const { eventBus } = useApp();
  const [isSubmittingTx, setIsSubmittingTx] = useState(false);
  const [indicator, setIndicator] = useState<keyof typeof Indicators>("spinner");

  const IndicatorComponent = Indicators[indicator];
  const WrapperComponent = as ?? "button";

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
    <WrapperComponent {...props}>
      <IndicatorComponent
        size="sm"
        color="current"
        className={twMerge({
          "stroke-2 stroke-status-success w-6 h-6": indicator === "success",
          "stroke-2 stroke-red-bean-400 w-6 h-6": indicator === "error",
        })}
      />
    </WrapperComponent>
  ) : (
    <>{children}</>
  );
};
