import { useCallback, useRef, useState } from "react";

import type { ChangeEvent } from "react";

type UseInputsOptions = {
  initialValues?: Record<string, string>;
};

type InputOptions = {
  validate?: (value: string) => boolean | string;
  mask?: (value: string, previousValue: string) => string;
};

type InputState = {
  value: string;
  error?: string;
  options?: InputOptions;
};

export function useInputs(options?: UseInputsOptions) {
  const { initialValues } = options || {};
  const [inputs, setInputs] = useState<Record<string, InputState>>(() =>
    initialValues
      ? Object.fromEntries(Object.entries(initialValues).map(([key, value]) => [key, { value }]))
      : {},
  );

  const inputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const inputOptions = useRef<Record<string, InputOptions>>({});

  const setValue = useCallback((name: string, newValue: string) => {
    setInputs((prev) => {
      const mask = inputOptions.current[name]?.mask;
      const maskedValue = mask ? mask(newValue, prev[name]?.value || "") : newValue;
      return prev[name]?.value !== maskedValue
        ? { ...prev, [name]: { ...prev[name], value: maskedValue } }
        : prev;
    });

    setError(name, undefined);
    const validate = inputOptions.current[name]?.validate;
    if (validate) {
      const validationResult = validate(newValue);
      setError(
        name,
        validationResult === true ? undefined : validationResult || "Value is not valid",
      );
    }
  }, []);

  const setError = useCallback((name: string, errorMessage?: string) => {
    setInputs((prev) => ({
      ...prev,
      [name]: {
        ...prev[name],
        error: errorMessage,
      },
    }));
  }, []);

  const reset = useCallback(() => {
    setInputs((prev) =>
      Object.fromEntries(
        Object.entries(prev).map(([key]) => [key, { value: initialValues?.[key] || "" }]),
      ),
    );
  }, []);

  const register = useCallback(
    (name: string, options?: InputOptions) => {
      inputOptions.current[name] = options || {};

      return {
        ref: (el: HTMLInputElement | null) => {
          if (el) inputRefs.current[name] = el;
        },
        name,
        value: inputs[name]?.value || "",
        errorMessage: inputs[name]?.error,
        onChange: (event: ChangeEvent<HTMLInputElement>) => setValue(name, event.target.value),
      };
    },
    [setValue, inputs],
  );

  return { register, setValue, setError, inputs, reset };
}
