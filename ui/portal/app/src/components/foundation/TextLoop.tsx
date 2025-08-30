import { useEffect, useMemo, useState } from "react";
import { View, Text } from "react-native";
import { AnimatePresence, MotiView } from "moti";
import { twMerge } from "@left-curve/foundation";

import type React from "react";

type TextLoopProps = {
  texts: string[];
  intervalMs?: number;
  className?: string;
};

export const TextLoop: React.FC<TextLoopProps> = ({ texts, intervalMs = 2000, className }) => {
  const [index, setIndex] = useState(0);

  const safeTexts = useMemo(() => (texts?.length ? texts : [""]), [texts]);

  useEffect(() => {
    const id = setInterval(() => {
      setIndex((prev) => (prev + 1) % safeTexts.length);
    }, intervalMs);
    return () => clearInterval(id);
  }, [intervalMs, safeTexts.length]);

  return (
    <View className="overflow-hidden relative min-h-[1.56rem] w-[8rem]">
      <AnimatePresence exitBeforeEnter>
        <MotiView
          key={index}
          className="absolute left-0"
          from={{ translateY: 20, opacity: 0 }}
          animate={{ translateY: 0, opacity: 1 }}
          exit={{ translateY: -20, opacity: 0 }}
          transition={{
            translateY: { type: "spring", damping: 20, stiffness: 300 },
            opacity: { type: "timing", duration: 500 },
          }}
        >
          <Text
            className={twMerge("text-primary-rice exposure-m-italic", className)}
            numberOfLines={1}
          >
            {safeTexts[index]}
          </Text>
        </MotiView>
      </AnimatePresence>
    </View>
  );
};
