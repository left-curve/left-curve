import { useTheme } from "~/hooks/useTheme";

import { Pressable, Text, ActivityIndicator, StyleSheet, View } from "react-native";
import { Shadow } from "react-native-shadow-2";
import { LinearGradient } from "expo-linear-gradient";

import { twMerge } from "@left-curve/applets-kit";
import { tv } from "tailwind-variants";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import type { PropsWithChildren } from "react";

export const iconColors = {
  light: {
    primary: "#FFFCF6",
    secondary: "#918CC6",
    tertiary: "#FFFCF6",
    "tertiary-red": "#ED4561",
    utility: "#9C4D21",
    link: "#918CC6",
    disabled: "#ACA9A7",
  },
  dark: {
    primary: "#2D2C2A",
    secondary: "#CBCBE7",
    tertiary: "#2D2C2A",
    "tertiary-red": "#FCCFD4",
    utility: "#E3BD66",
    link: "#CBCBE7",
    disabled: "#807D78",
  },
} as const;

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
  base: "flex items-center justify-center overflow-hidden rounded-full transition-all duration-200",
  variants: {
    variant: {
      primary: "bg-red-bean-400",
      secondary: "bg-primary-blue",
      tertiary: "bg-button-green",
      "tertiary-red": "bg-surface-primary-red",
      utility: "bg-surface-quaternary-rice rounded-md",
      link: "bg-transparent",
    },
    size: {
      xs: "h-[25px] py-1 px-[6px] exposure-xs-italic text-xs gap-[2px]",
      sm: "h-[32px] py-[6px] px-2 exposure-sm-italic gap-[2px]",
      md: "h-[40px] py-[10px] px-3 exposure-sm-italic text-md gap-[4px]",
      lg: "h-[44px] py-[11px] px-3 exposure-m-italic text-lg gap-[4px]",
      xl: "h-[56px] py-[14px] px-4 exposure-l-italic text-h4 gap-[6px]",
    },
    radius: {
      none: "rounded-none",
      sm: "rounded-sm",
      md: "rounded-md",
      lg: "rounded-lg",
      xl: "rounded-xl",
      full: "rounded-full",
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

const textVariants = tv({
  base: "exposure-sm-italic",
  variants: {
    variant: {
      primary: "text-surface-primary-rice",
      secondary: "text-secondary-blue",
      tertiary: "text-surface-primary-rice",
      "tertiary-red": "text-tertiary-red",
      utility: "text-secondary-rice",
      link: "text-secondary-blue",
    },
    isDisabled: {
      true: "text-foreground-disabled-gray",
    },
    size: {
      xs: "exposure-xs-italic text-xs",
      sm: "exposure-sm-italic",
      md: "exposure-sm-italic text-md",
      lg: "exposure-m-italic text-lg",
      xl: "exposure-l-italic text-h4",
    },
  },
  defaultVariants: {
    variant: "primary",
    size: "md",
  },
});

interface ButtonTextProps extends VariantProps<typeof textVariants> {
  children: React.ReactNode;
  isDisabled?: boolean;
}

const ButtonText: React.FC<PropsWithChildren<ButtonTextProps>> = ({
  children,
  variant,
  size,
  isDisabled,
}) => {
  return <Text className={textVariants({ variant, size, isDisabled })}>{children}</Text>;
};

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
}) => {
  const styles = buttonVariants({ variant, size, isDisabled, radius });

  return (
    <ButtonShadow radius={radius} variant={variant}>
      <Pressable className={twMerge(styles)} disabled={isDisabled || isLoading} onPress={onPress}>
        {isLoading ? (
          <ActivityIndicator color="white" size="small" />
        ) : (
          <View className="flex flex-row items-center gap-2 justify-center">
            {leftIcon}
            <ButtonText variant={variant} size={size} isDisabled={isDisabled}>
              {children}
            </ButtonText>
            {rightIcon}
          </View>
        )}
      </Pressable>
    </ButtonShadow>
  );
};
