"use client";

import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";
import React, {
  cloneElement,
  createContext,
  memo,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

type Handler = (() => Promise<void>) | (() => void) | null;

type WizardValues<T = unknown> = {
  /**
   * Reset the wizard to the initial state
   */
  reset: () => void;
  /**
   * It trigger the onFinish callback if provided and reset the state
   */
  done: () => void;
  /**
   * Go to the next step
   */
  nextStep: () => Promise<void>;
  /**
   * Go to the previous step
   */
  previousStep: () => void;
  /**
   * Go to the given step index
   * @param stepIndex The step index, starts at 0
   */
  goToStep: (stepIndex: number) => void;
  /**
   * Attach a callback that will be called when calling `nextStep()`
   * @param handler Can be either sync or async
   */
  onStepLeave: (handler: Handler) => void;
  /**
   * Update the data passed between steps
   * @param data The data to pass
   */
  setData: (data: Partial<T>) => void;
  /**
   * Indicate the current state of the handler
   *
   * Will reflect the handler promise state: will be `true` if the handler promise is pending and
   * `false` when the handler is either fulfilled or rejected
   */
  isLoading: boolean;
  /** The current active step of the wizard */
  activeStep: number;
  /** The total number of steps of the wizard */
  stepCount: number;
  /** Indicate if the current step is the first step (aka no previous step) */
  isFirstStep: boolean;
  /** Indicate if the current step is the last step (aka no next step) */
  isLastStep: boolean;
  /** The data passed between steps */
  data: T;
};

const WizardContext = createContext<WizardValues | null>(null);

export function useWizard<T = any>(): WizardValues<T> {
  const context = useContext(WizardContext);
  if (!context) throw Error("useWizard must be used within a WizardProvider");
  return context as WizardValues<T>;
}

interface Props {
  wrapper?: React.ReactElement;
  /** Callback that will be invoked when the wizard is reset */
  onReset?: () => void;
  /** Callback that will be invoked when the wizard is done */
  onFinish?: () => void;
  /** Callback that will be invoked with the new step index when the wizard changes steps */
  onStepChange?: (stepIndex: number) => void;
  /** Optional start index @default 0 */
  startIndex?: number;
  /** Persist key */
  persistKey?: string;
  /** Load data when mounting */
  defaultData?: unknown;
}

export const WizardProvider: React.FC<React.PropsWithChildren<Props>> = memo(
  ({
    children,
    onStepChange,
    wrapper: Wrapper,
    startIndex = 0,
    persistKey,
    defaultData,
    onReset,
    onFinish,
  }) => {
    const [activeStep, setActiveStep] = useState(startIndex);
    const [isLoading, setIsLoading] = useState(false);
    const [data, setData] = useState<unknown>(defaultData ?? {});
    const hasNextStep = useRef(true);
    const hasPreviousStep = useRef(false);
    const nextStepHandler = useRef<Handler>(() => {});
    const reactChildren = React.Children.toArray(children);
    const stepCount = reactChildren.length;

    hasNextStep.current = activeStep < stepCount - 1;
    hasPreviousStep.current = activeStep > 0;

    useEffect(() => {
      if (persistKey) {
        const item = localStorage.getItem(persistKey);
        if (item) {
          const { step, data } = deserializeJson<{ step: number; data: unknown }>(item);
          setActiveStep(step);
          setData(data);
        }
      }
    }, []);

    useEffect(() => {
      if (persistKey) {
        localStorage.setItem(persistKey, serializeJson({ step: activeStep, data }));
      }
    }, [data, activeStep]);

    const goToNextStep = useCallback(() => {
      if (hasNextStep.current) {
        const newActiveStepIndex = activeStep + 1;

        setActiveStep(newActiveStepIndex);
        onStepChange?.(newActiveStepIndex);
      }
      if (window?.document.activeElement instanceof HTMLElement) {
        window?.document.activeElement.blur();
      }
    }, [activeStep, onStepChange]);

    const goToPreviousStep = useCallback(() => {
      if (hasPreviousStep.current) {
        nextStepHandler.current = null;
        const newActiveStepIndex = activeStep - 1;

        setActiveStep(newActiveStepIndex);
        onStepChange?.(newActiveStepIndex);
      }
      if (window?.document.activeElement instanceof HTMLElement) {
        window?.document?.activeElement.blur();
      }
    }, [activeStep, onStepChange]);

    const goToStep = useCallback(
      (stepIndex: number) => {
        if (stepIndex >= 0 && stepIndex < stepCount) {
          nextStepHandler.current = null;
          setActiveStep(stepIndex);
          onStepChange?.(stepIndex);
        }
      },
      [stepCount, onStepChange],
    );

    // Callback to attach the step handler
    const handleStep = useRef((handler: Handler) => {
      nextStepHandler.current = handler;
    });

    const doNextStep = useCallback(async () => {
      if (hasNextStep.current && nextStepHandler.current) {
        try {
          setIsLoading(true);
          await nextStepHandler.current();
          setIsLoading(false);
          nextStepHandler.current = null;
          goToNextStep();
        } catch (error) {
          setIsLoading(false);
          throw error;
        }
      } else {
        goToNextStep();
      }
    }, [goToNextStep]);

    const doReset = useCallback(() => {
      setActiveStep(startIndex);
      setData({});
      if (persistKey) {
        localStorage.removeItem(persistKey);
      }
      onReset?.();
    }, [startIndex, onReset]);

    const doFinish = useCallback(() => {
      onFinish?.();
      doReset();
    }, [onFinish, doReset]);

    const wizardValue = useMemo(
      () => ({
        reset: doReset,
        done: doFinish,
        nextStep: doNextStep,
        previousStep: goToPreviousStep,
        onStepLeave: handleStep.current,
        setData,
        goToStep,
        isLoading,
        activeStep,
        stepCount,
        data,
        isFirstStep: !hasPreviousStep.current,
        isLastStep: !hasNextStep.current,
      }),
      [
        doReset,
        doFinish,
        doNextStep,
        goToPreviousStep,
        isLoading,
        activeStep,
        stepCount,
        data,
        goToStep,
      ],
    );

    const activeStepContent = useMemo(() => {
      return reactChildren[activeStep];
    }, [activeStep, reactChildren]);

    const enhancedActiveStepContent = useMemo(
      () => (Wrapper ? cloneElement(Wrapper, { children: activeStepContent }) : activeStepContent),
      [Wrapper, activeStepContent],
    );

    return (
      <WizardContext.Provider value={wizardValue}>
        {enhancedActiveStepContent}
      </WizardContext.Provider>
    );
  },
);
