import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";

interface TextLoopProps {
  texts: string[];
}

export const TextLoop: React.FC<TextLoopProps> = ({ texts }) => {
  const [index, setIndex] = useState<number>(0);

  useEffect(() => {
    setTimeout(() => {
      const next = index + 1;
      setIndex(next % texts.length);
    }, 2 * 1000);
  }, [index, setIndex, texts]);

  return (
    <span className="overflow-hidden relative min-h-[1.56rem] w-[8rem]">
      <AnimatePresence initial={false}>
        <motion.span
          className="absolute left-0 w-[2rem] exposure-m-italic text-rice-800"
          key={index}
          layout
          variants={{
            enter: {
              translateY: 20,
              opacity: 0,
              height: 0,
            },
            center: {
              zIndex: 1,
              translateY: 0,
              opacity: 1,
              height: "auto",
            },
            exit: {
              zIndex: 0,
              translateY: -20,
              opacity: 0,
              height: 0,
            },
          }}
          initial="enter"
          animate="center"
          exit="exit"
          transition={{
            translateY: { type: "spring", stiffness: 1000, damping: 200 },
            opacity: { duration: 0.5 },
          }}
        >
          {texts[index]}
        </motion.span>
      </AnimatePresence>
    </span>
  );
};
