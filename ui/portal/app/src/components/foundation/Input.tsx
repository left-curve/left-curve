import type React from "react";
import { forwardRef, useState } from "react";
import { TextInput, View, Text, TouchableWithoutFeedback, Keyboard } from "react-native";
import { tv, type VariantProps } from "tailwind-variants";
import { twMerge } from "@left-curve/applets-kit";
import { GlobalText } from "./GlobalText";
import { useTheme } from "~/hooks/useTheme";
import { ShadowContainer } from "./ShadowContainer";

export interface InputProps
  extends React.ComponentPropsWithoutRef<typeof TextInput>,
    VariantProps<typeof inputVariants> {
  label?: React.ReactNode;
  startContent?: React.ReactNode;
  endContent?: React.ReactNode;
  bottomComponent?: React.ReactNode;
  insideBottomComponent?: React.ReactNode;
  errorMessage?: string;
  hideErrorMessage?: boolean;
  hintMessage?: string;
  isLoading?: boolean;
  classNames?: {
    base?: string;
    inputParent?: string;
    inputWrapper?: string;
    input?: string;
  };
}

export const Input = forwardRef<TextInput, InputProps>(
  (
    {
      classNames,
      startContent,
      endContent,
      bottomComponent,
      insideBottomComponent,
      label,
      isInvalid: invalid,
      isDisabled,
      isLoading,
      fullWidth,
      placeholder,
      errorMessage,
      hideErrorMessage,
      hintMessage,
      onFocus,
      onBlur,
      ...props
    },
    ref,
  ) => {
    const { theme } = useTheme();
    const [isFocus, setIsFocus] = useState(false);
    const isInvalid = !!errorMessage || invalid;

    const { base, input, inputParent, inputWrapper } = inputVariants({
      fullWidth,
      isDisabled,
      isInvalid,
    });

    return (
      <TouchableWithoutFeedback onPress={Keyboard.dismiss} accessible={false}>
        <ShadowContainer>
          <View className={base({ className: classNames?.base })}>
            {label ? (
              typeof label === "string" ? (
                <Text className="exposure-sm-italic text-ink-secondary-700 mb-1">{label}</Text>
              ) : (
                label
              )
            ) : null}

            <View
              className={twMerge(
                inputWrapper({ className: classNames?.inputWrapper }),
                isFocus ? "border-surface-quaternary-rice" : "",
              )}
            >
              <View className={inputParent({ className: classNames?.inputParent })}>
                {startContent ? <View>{startContent}</View> : null}

                <View className="relative flex-1 flex-row items-center">
                  <TextInput
                    style={{ borderWidth: 0 }}
                    ref={ref}
                    editable={!isDisabled}
                    placeholder={typeof placeholder === "string" ? placeholder : undefined}
                    placeholderTextColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
                    selectionColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
                    onFocus={(e) => {
                      setIsFocus(true);
                      onFocus?.(e);
                    }}
                    onBlur={(e) => {
                      setIsFocus(false);
                      onBlur?.(e);
                    }}
                    className={input({ className: classNames?.input })}
                    {...props}
                  />
                </View>

                {endContent ? <View>{endContent}</View> : null}
              </View>

              {insideBottomComponent ? (
                <View className="absolute bottom-1 left-0 right-0">{insideBottomComponent}</View>
              ) : null}
            </View>

            {!hideErrorMessage && errorMessage ? (
              <GlobalText className="diatype-sm-regular text-status-fail mt-1">
                {errorMessage}
              </GlobalText>
            ) : null}

            {!bottomComponent && hintMessage ? (
              <GlobalText className="diatype-sm-regular text-ink-tertiary-500 mt-1">
                {hintMessage}
              </GlobalText>
            ) : null}

            {bottomComponent ? (
              <View className="text-ink-tertiary-500 diatype-sm-regular mt-1">
                {bottomComponent}
              </View>
            ) : null}
          </View>
        </ShadowContainer>
      </TouchableWithoutFeedback>
    );
  },
);

Input.displayName = "Input";

const inputVariants = tv(
  {
    slots: {
      base: "flex flex-col gap-1 relative text-ink-secondary-700",
      inputWrapper:
        "relative w-full flex flex-row items-center gap-2 px-2 py-[6fspx] rounded-lg h-[46px] bg-surface-secondary-rice border border-transparent",
      inputParent: "w-full flex flex-row items-center gap-2",
      input:
        "border-0 flex-1 diatype-m-regular bg-transparent text-ink-secondary-700 placeholder:text-ink-tertiary-500",
    },
    variants: {
      isDisabled: {
        true: {
          base: "opacity-50",
          inputWrapper: "bg-surface-disabled-gray border-transparent",
        },
      },
      isInvalid: {
        true: {
          inputWrapper: "border-status-fail",
          input: "text-ink-secondary-700",
        },
      },
      fullWidth: {
        true: {
          base: "w-full",
        },
      },
    },
    defaultVariants: {
      fullWidth: true,
      isDisabled: false,
    },
  },
  { twMerge: true },
);
