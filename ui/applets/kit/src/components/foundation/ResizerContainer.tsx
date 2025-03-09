import { motion } from "framer-motion";
import type React from "react";

interface ResizerContainerProps {
  children?: React.ReactNode;
  className?: string;
}

export const ResizerContainer: React.FC<ResizerContainerProps> = ({ children, className = "" }) => {
  return (
    <motion.div
      layout
      layoutId="resizer"
      className={className}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.1 }}
    >
      {children}
    </motion.div>
  );
};
