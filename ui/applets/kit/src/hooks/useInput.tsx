import { useCallback, useRef, useState } from "react";

import type { ChangeEvent } from "react";

type UseInputOptions = {
  initialValue?: string;
};

type InputOptions = {
  validate?: (value: string) => boolean | string;
  mask?: (value: string) => string;
};

export function useInput(name: string, options?: UseInputOptions) {
  const { initialValue } = options || {};

  const [value, setValue] = useState(() => initialValue || "");
  const [error, setError] = useState<string | undefined>();
  const inputRef = useRef<HTMLInputElement>(null);
  const inputOptions = useRef<InputOptions | null>(null);

  const handleChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const { validate, mask } = inputOptions.current || {};
      const newValue = mask ? mask(event.target.value) : event.target.value;
      setValue(newValue);

      if (validate) {
        const validationResult = validate(newValue);
        setError(
          validationResult === true
            ? undefined
            : validationResult || "Value introduced is not valid",
        );
      }
    },
    [value, setValue],
  );

  const register = useCallback(
    (parameters?: InputOptions) => {
      const { validate, mask } = parameters || {};
      inputOptions.current = { validate, mask };
      return {
        ref: inputRef,
        name,
        value,
        errorMessage: error,
        onChange: handleChange,
      };
    },
    [handleChange, name, error, value],
  );

  return { register, error, setError, value };
}
