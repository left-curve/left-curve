import type React from "react";
import { useState } from "react";
import { Pressable } from "react-native";
import * as Clipboard from "expo-clipboard";

import { IconCopyCheck } from "./icons/IconCopyCheck";
import { IconCopyNoCheck } from "./icons/IconCopyNoCheck";
import { twMerge } from "@left-curve/foundation";

type TextCopyProps = {
  copyText?: string;
  className?: string;
};

export const TextCopy: React.FC<TextCopyProps> = ({ copyText, className }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    if (copyText) {
      await Clipboard.setStringAsync(copyText);
      setCopied(true);
      setTimeout(() => setCopied(false), 1000);
    }
  };

  return (
    <Pressable
      onPress={handleCopy}
      className={className}
      accessibilityRole="button"
      accessibilityLabel="Copy to clipboard"
    >
      {copied ? (
        <IconCopyCheck className={twMerge("w-4 h-4", className)} />
      ) : (
        <IconCopyNoCheck className={twMerge("w-4 h-4", className)} />
      )}
    </Pressable>
  );
};
