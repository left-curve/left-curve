"use client";

import React, {
	cloneElement,
	createContext,
	memo,
	useCallback,
	useContext,
	useMemo,
	useRef,
	useState,
} from "react";

type Handler = (() => Promise<void>) | (() => void) | null;

type WizardValues = {
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
	 * @param data Any data to pass between steps
	 */
	handleStep: (handler: Handler, data?: unknown) => void;
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
	// biome-ignore lint/suspicious/noExplicitAny: The data can be of any type
	data: any;
};

const WizardContext = createContext<WizardValues | null>(null);

export const useWizard = () => {
	const context = useContext(WizardContext);
	if (!context) throw Error("useWizard must be used within a WizardProvider");
	return context;
};

interface Props {
	wrapper?: React.ReactElement;
	/** Callback that will be invoked with the new step index when the wizard changes steps */
	onStepChange?: (stepIndex: number) => void;
	/** Optional start index @default 0 */
	startIndex?: number;
}

export const WizardContainer: React.FC<React.PropsWithChildren<Props>> = memo(
	({ children, onStepChange, wrapper: Wrapper, startIndex = 0 }) => {
		const [activeStep, setActiveStep] = useState(startIndex || 0);
		const [isLoading, setIsLoading] = useState(false);
		const wizardData = useRef<unknown>(null);
		const hasNextStep = useRef(true);
		const hasPreviousStep = useRef(false);
		const nextStepHandler = useRef<Handler>(() => {});
		const stepCount = React.Children.toArray(children).length;

		hasNextStep.current = activeStep < stepCount - 1;
		hasPreviousStep.current = activeStep > 0;

		const goToNextStep = useCallback(() => {
			if (hasNextStep.current) {
				const newActiveStepIndex = activeStep + 1;

				setActiveStep(newActiveStepIndex);
				onStepChange?.(newActiveStepIndex);
			}
		}, [activeStep, onStepChange]);

		const goToPreviousStep = useCallback(() => {
			if (hasPreviousStep.current) {
				nextStepHandler.current = null;
				const newActiveStepIndex = activeStep - 1;

				setActiveStep(newActiveStepIndex);
				onStepChange?.(newActiveStepIndex);
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
		const handleStep = useRef((handler: Handler, payload?: unknown) => {
			nextStepHandler.current = handler;
			if (payload) wizardData.current = payload;
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

		const wizardValue = useMemo(
			() => ({
				nextStep: doNextStep,
				previousStep: goToPreviousStep,
				handleStep: handleStep.current,
				isLoading,
				activeStep,
				stepCount,
				data: wizardData.current,
				isFirstStep: !hasPreviousStep.current,
				isLastStep: !hasNextStep.current,
				goToStep,
			}),
			[
				doNextStep,
				goToPreviousStep,
				isLoading,
				activeStep,
				stepCount,
				goToStep,
			],
		);

		const activeStepContent = useMemo(() => {
			const reactChildren = React.Children.toArray(children);

			return reactChildren[activeStep];
		}, [activeStep, children]);

		const enhancedActiveStepContent = useMemo(
			() =>
				Wrapper
					? cloneElement(Wrapper, { children: activeStepContent })
					: activeStepContent,
			[Wrapper, activeStepContent],
		);

		return (
			<WizardContext.Provider value={wizardValue}>
				{enhancedActiveStepContent}
			</WizardContext.Provider>
		);
	},
);
