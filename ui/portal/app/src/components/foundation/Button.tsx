import { useTheme } from "~/hooks/useTheme";

import { Shadow } from "react-native-shadow-2";
import { LinearGradient } from "expo-linear-gradient";
import { Pressable, Text, ActivityIndicator, StyleSheet, View } from "react-native";

import { tv } from "tailwind-variants";
import { twMerge } from "@left-curve/foundation";
import { cloneElement, isValidElement } from "react";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import type { PropsWithChildren, ReactElement, ReactNode } from "react";

export const radiusSizes = {
  none: 0,
  xxs: 4,
  xs: 8,
  sm: 10,
  md: 12,
  lg: 14,
  xl: 16,
  "2xl": 18,
  "3xl": 20,
  "4xl": 22,
  "5xl": 24,
  full: 9999,
};

const buttonVariants = tv({
  slots: {
    base: "flex items-center justify-center overflow-hidden gap-2 rounded-full transition-all duration-200",
    icons: "",
    text: "",
  },
  variants: {
    variant: {
      primary: {
        base: "bg-red-bean-400",
        icons: "text-surface-primary-rice",
        text: "text-surface-primary-rice",
      },
      secondary: {
        base: "bg-primary-blue",
        icons: "text-secondary-blue",
        text: "text-secondary-blue",
      },
      tertiary: {
        base: "bg-button-green",
        icons: "text-surface-primary-rice",
        text: "text-surface-primary-rice",
      },
      "tertiary-red": {
        base: "bg-surface-primary-red",
        icons: "text-tertiary-red",
        text: "text-tertiary-red",
      },
      utility: {
        base: "bg-surface-quaternary-rice !rounded-md",
        icons: "text-secondary-rice",
        text: "text-secondary-rice",
      },
      link: {
        base: "bg-transparent",
        icons: "text-secondary-blue",
        text: "text-secondary-blue",
      },
    },
    radius: {
      none: {
        base: "rounded-none",
      },
      sm: {
        base: "rounded-sm",
      },
      md: {
        base: "rounded-md",
      },
      lg: {
        base: "rounded-lg",
      },
      xl: {
        base: "rounded-xl",
      },
      full: {
        base: "rounded-full",
      },
    },
    size: {
      xs: {
        base: "h-[25px] py-1 px-[6px]",
        icons: "",
        text: "exposure-xs-italic text-xs",
      },
      sm: {
        base: "h-[32px] py-[6px] px-2",
        icons: "",
        text: "exposure-sm-italic",
      },
      md: {
        base: "h-[40px] py-[8px] px-3",
        icons: "",
        text: "exposure-sm-italic text-md",
      },
      lg: {
        base: "h-[44px] py-[11px] px-3",
        icons: "",
        text: "exposure-m-italic text-lg",
      },
      xl: {
        base: "h-[56px] py-[14px] px-4",
        icons: "",
        text: "exposure-l-italic text-h4",
      },
      icon: {
        base: "p-[10px] h-[44px] w-[44px]",
        icons: "h-8 w-8",
      },
    },
    isDisabled: {
      true: "bg-surface-disabled-gray opacity-50",
    },
  },
  defaultVariants: {
    size: "md",
    variant: "primary",
    isDisabled: false,
  },
});

const ButtonShadow: React.FC<PropsWithChildren<Pick<ButtonProps, "radius" | "variant">>> = ({
  children,
  radius = "full",
  variant,
}) => {
  const { theme } = useTheme();
  const br = radiusSizes[radius];

  if (variant === "link") {
    return <>{children}</>;
  }

  if (variant === "tertiary-red") {
    return (
      <Shadow
        distance={3}
        startColor="rgba(0,0,0,0.07)"
        offset={[0, -1]}
        style={{ borderRadius: br }}
      >
        <Shadow
          distance={3}
          startColor="rgba(255,255,255,0.07)"
          offset={[0, 2]}
          style={{ borderRadius: br }}
        >
          <Shadow
            distance={1}
            startColor="rgba(0,0,0,0.04)"
            offset={[0, 1]}
            style={{ borderRadius: br }}
          >
            <LinearGradient
              colors={["rgba(255,255,255,0.12)", "transparent"]}
              start={{ x: 0.5, y: 0 }}
              end={{ x: 0.5, y: 0.6 }}
              style={{ ...StyleSheet.absoluteFillObject, borderRadius: br }}
            />
            {children}
          </Shadow>
        </Shadow>
      </Shadow>
    );
  }

  if (theme === "light") {
    return (
      <Shadow
        distance={4}
        startColor="rgba(171,158,138,0.40)"
        offset={[0, 2]}
        style={{ borderRadius: br }}
      >
        <Shadow
          distance={2}
          startColor="rgba(241,219,186,0.50)"
          offset={[0, -1]}
          style={{ borderRadius: br }}
        >
          <LinearGradient
            colors={["rgba(255,255,255,0.64)", "rgba(255,255,255,0.00)"]}
            start={{ x: 0.5, y: 0 }}
            end={{ x: 0.5, y: 0.7 }}
            style={{ ...StyleSheet.absoluteFillObject, borderRadius: br }}
          />
          <LinearGradient
            colors={["rgba(255,255,255,0.48)", "rgba(255,255,255,0.00)"]}
            start={{ x: 0.5, y: 0.3 }}
            end={{ x: 0.5, y: 1 }}
            style={{ ...StyleSheet.absoluteFillObject, borderRadius: br }}
          />
          {children}
        </Shadow>
      </Shadow>
    );
  }

  return (
    <Shadow distance={6} startColor="rgba(0,0,0,0.04)" offset={[0, 4]} style={{ borderRadius: br }}>
      <Shadow
        distance={6}
        startColor="rgba(0,0,0,0.04)"
        offset={[0, 4]}
        style={{ borderRadius: br }}
      >
        <LinearGradient
          colors={["rgba(255,255,255,0.48)", "rgba(255,255,255,0.00)"]}
          start={{ x: 0.5, y: 0.2 }}
          end={{ x: 0.5, y: 1 }}
          style={{ ...StyleSheet.absoluteFillObject, borderRadius: br }}
        />
        <LinearGradient
          colors={["rgba(255,255,255,0.64)", "rgba(255,255,255,0.00)"]}
          start={{ x: 0.5, y: 0 }}
          end={{ x: 0.5, y: 0.6 }}
          style={{ ...StyleSheet.absoluteFillObject, borderRadius: br }}
        />
        {children}
      </Shadow>
    </Shadow>
  );
};
export interface ButtonProps extends VariantProps<typeof buttonVariants> {
  isLoading?: boolean;
  isDisabled?: boolean;
  onPress?: () => void;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
  className?: string;
  classNames?: {
    base?: string;
    icons?: string;
    text?: string;
  };
}

export const Button: React.FC<PropsWithChildren<ButtonProps>> = ({
  variant,
  size,
  isDisabled,
  isLoading,
  children,
  onPress,
  radius = "full",
  leftIcon,
  rightIcon,
  classNames,
}) => {
  const styles = buttonVariants({ variant, size, isDisabled, radius });
  const renderIcon = (node?: ReactNode) =>
    isValidElement(node)
      ? cloneElement(node as ReactElement, {
          className: twMerge(styles.icons(), classNames?.icons),
        })
      : null;

  const styledLeftIcon = renderIcon(leftIcon);
  const styledRightIcon = renderIcon(rightIcon);

  const childrenComponent =
    children && isValidElement(children)
      ? cloneElement(children as ReactElement, {
          className: twMerge(styles.text(), classNames?.text),
        })
      : children;

  return (
    <ButtonShadow radius={radius} variant={variant}>
      <Pressable
        disabled={isDisabled || isLoading}
        onPress={onPress}
        className={twMerge("flex flex-row items-center justify-center", classNames?.base)}
      >
        {isLoading ? (
          <ActivityIndicator color="white" size="small" />
        ) : (
          <View
            className={twMerge(
              "flex flex-row items-center justify-center",
              styles.base(),
              classNames?.base,
            )}
          >
            {styledLeftIcon}

            {children ? (
              <Text className={twMerge(styles.text(), classNames?.text)}>{childrenComponent}</Text>
            ) : null}

            {styledRightIcon}
          </View>
        )}
      </Pressable>
    </ButtonShadow>
  );
};
