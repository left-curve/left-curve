import { useEffect, useMemo, useState, useRef } from "react";

import { View, Text, Animated } from "react-native";

import { twMerge } from "@left-curve/foundation";

import type React from "react";

type TextLoopProps = {
  texts: string[];
  intervalMs?: number;
  className?: string;
};

export const TextLoop: React.FC<TextLoopProps> = ({ texts, intervalMs = 2000, className }) => {
  const [index, setIndex] = useState(0);
  const fadeAnim = useRef(new Animated.Value(1)).current;
  const translateY = useRef(new Animated.Value(0)).current;

  const safeTexts = useMemo(() => (texts?.length ? texts : [""]), [texts]);

  useEffect(() => {
    const animate = () => {
      Animated.parallel([
        Animated.timing(fadeAnim, { toValue: 0, duration: 150, useNativeDriver: true }),
        Animated.timing(translateY, { toValue: -10, duration: 150, useNativeDriver: true }),
      ]).start(() => {
        setIndex((prev) => (prev + 1) % safeTexts.length);
        translateY.setValue(10);
        Animated.parallel([
          Animated.timing(fadeAnim, { toValue: 1, duration: 150, useNativeDriver: true }),
          Animated.timing(translateY, { toValue: 0, duration: 150, useNativeDriver: true }),
        ]).start();
      });
    };

    const id = setInterval(animate, intervalMs);
    return () => clearInterval(id);
  }, [intervalMs, safeTexts.length, fadeAnim, translateY]);

  return (
    <View className="overflow-hidden relative min-h-[1.56rem] w-[8rem]">
      <Animated.View
        style={{
          opacity: fadeAnim,
          transform: [{ translateY }],
        }}
      >
        <Text
          className={twMerge("text-ink-secondary-rice exposure-m-italic", className)}
          numberOfLines={1}
        >
          {safeTexts[index]}
        </Text>
      </Animated.View>
    </View>
  );
};
