import { useCallback, useMemo, useRef, useState } from "react";

import type { ChangeEvent } from "react";

type UseInputsOptions = {
  initialValues?: Record<string, string>;
  strategy?: ValidateStrategy;
};

type ValidateStrategy = "onChange" | "onSubmit";

type InputOptions = {
  strategy?: ValidateStrategy;
  validate?: (value: string) => boolean | string;
  mask?: (value: string, previousValue: string) => string;
};

type InputState = {
  value: string;
  error?: string;
  options?: InputOptions;
};

export function useInputs(options: UseInputsOptions = {}) {
  const { initialValues, strategy: defaultStrategy = "onSubmit" } = options;
  const [inputs, setInputs] = useState<Record<string, InputState>>(() =>
    initialValues
      ? Object.fromEntries(Object.entries(initialValues).map(([key, value]) => [key, { value }]))
      : {},
  );

  const errors = useMemo(() => {
    return Object.entries(inputs).reduce((acc, [key, input]) => {
      if (input.error) acc[key] = input.error;
      return acc;
    }, Object.create({}));
  }, [inputs]);

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
    const strategy = inputOptions.current[name].strategy;
    if (validate && strategy === "onChange") {
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
    (name: string, options: InputOptions = {}) => {
      const { strategy = defaultStrategy, mask, validate } = options;
      inputOptions.current[name] = {
        validate,
        mask,
        strategy,
      };

      return {
        ref: (el: HTMLInputElement | null) => {
          if (el) inputRefs.current[name] = el;
        },
        name,
        value: inputs[name]?.value || "",
        errorMessage: inputs[name]?.error,
        onChange: (event: ChangeEvent<HTMLInputElement> | string) =>
          setValue(name, typeof event === "string" ? event : event.target.value),
      };
    },
    [setValue, inputs],
  );

  const revalidate = useCallback((name?: string) => {
    const options = inputOptions.current || {};
    const inputNames = name ? [name] : Object.keys(options);

    inputNames.forEach((key) => {
      const value = inputRefs.current[key as keyof typeof inputRefs]?.value || "";
      const { validate } = options[key];
      if (validate) {
        const validationResult = validate(value);
        setError(
          key,
          validationResult === true ? undefined : validationResult || "Value is not valid",
        );
      }
    });
  }, []);

  const handleSubmit = useCallback(<T>(fn: (data: T) => void) => {
    return (e: React.FormEvent<HTMLFormElement>) => {
      e.preventDefault();

      let shouldSubmit = true;
      const options = inputOptions.current || {};

      const formData = Object.entries(options).map(([key, { validate }]) => {
        const value = inputRefs.current[key as keyof typeof inputRefs]?.value || "";
        if (validate) {
          const validationResult = validate(value);
          if (validationResult !== true) {
            setError(key, validationResult || "Value is not valid");
            shouldSubmit = false;
          }
        }
        return [key, value];
      });

      if (shouldSubmit) fn(Object.fromEntries(formData) as T);
    };
  }, []);

  const isValid = !Object.keys(errors).length;

  return { register, setValue, setError, inputs, errors, isValid, reset, handleSubmit, revalidate };
}
