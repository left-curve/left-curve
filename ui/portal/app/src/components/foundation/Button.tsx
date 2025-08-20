import { Pressable, Text, ActivityIndicator, StyleSheet } from "react-native";
import { tv } from "tailwind-variants";
import { twMerge } from "@left-curve/applets-kit";
import { Shadow } from "react-native-shadow-2";
import React, { PropsWithChildren } from "react";
import { LinearGradient } from "expo-linear-gradient";

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
  base: "exposure-sm-italic pb-2",
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

type ButtonTextProps = {
  children: React.ReactNode;
  variant?: keyof typeof textVariants.variants.variant;
  size?: keyof typeof textVariants.variants.size;
  isDisabled?: boolean;
};

const ButtonText: React.FC<ButtonTextProps> = ({ children, variant, size, isDisabled }) => {
  return <Text className={textVariants({ variant, size, isDisabled })}>{children}</Text>;
};

const ButtonShadow: React.FC<
  PropsWithChildren<{
    radius: keyof typeof buttonVariants.variants.radius;
    variant?: keyof typeof buttonVariants.variants.variant;
  }>
> = ({ children, radius, variant }) => {
  if (variant?.includes("link")) {
    return <>{children}</>;
  }

  if (variant === "tertiary-red") {
    return (
      <Shadow
        distance={3}
        startColor="rgba(0, 0, 0, 0.07)"
        offset={[0, -1]}
        style={{ borderRadius: radiusSizes[radius] }}
      >
        <Shadow
          distance={3}
          startColor="rgba(255, 255, 255, 0.07)"
          offset={[0, 2]}
          style={{ borderRadius: radiusSizes[radius] }}
        >
          <Shadow
            distance={1}
            startColor="rgba(0, 0, 0, 0.04)"
            offset={[0, 1]}
            style={{ borderRadius: radiusSizes[radius] }}
          >
            <LinearGradient
              colors={["rgba(0,0,0,0.07)", "transparent"]}
              style={{ ...StyleSheet.absoluteFillObject, borderRadius: radiusSizes[radius] }}
            />
            {children}
          </Shadow>
        </Shadow>
      </Shadow>
    );
  }

  return (
    <Shadow
      distance={4}
      startColor="rgba(171, 158, 138, 0.4)"
      offset={[0, 2]}
      style={{ borderRadius: radiusSizes[radius] }}
    >
      <Shadow
        distance={2}
        startColor="rgba(241, 219, 186, 0.5)"
        offset={[0, -1]}
        style={{ borderRadius: radiusSizes[radius] }}
      >
        <LinearGradient
          colors={["rgba(255,255,255,0.64)", "rgba(255,255,255,0.48)", "transparent"]}
          style={[{ ...StyleSheet.absoluteFillObject, borderRadius: radiusSizes[radius] }]}
        />
        {children}
      </Shadow>
    </Shadow>
  );
};

export type ButtonProps = {
  variant?: keyof typeof buttonVariants.variants.variant;
  size?: keyof typeof buttonVariants.variants.size;
  radius?: keyof typeof buttonVariants.variants.radius;
  isDisabled?: boolean;
  isLoading?: boolean;
  children: React.ReactNode;
  onPress?: () => void;
};

export const Button: React.FC<ButtonProps> = ({
  variant,
  size,
  isDisabled,
  isLoading,
  children,
  onPress,
  radius = "full",
}) => {
  const styles = buttonVariants({ variant, size, isDisabled, radius });

  return (
    <ButtonShadow radius={radius} variant={variant}>
      <Pressable className={twMerge(styles)} disabled={isDisabled || isLoading} onPress={onPress}>
        {isLoading ? (
          <ActivityIndicator color="white" size="small" />
        ) : (
          <ButtonText variant={variant} size={size} isDisabled={isDisabled}>
            {children}
          </ButtonText>
        )}
      </Pressable>
    </ButtonShadow>
  );
};
