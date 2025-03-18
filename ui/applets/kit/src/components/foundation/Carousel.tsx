import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import { IconChevronLeftCarousell, IconChevronRight } from "../";
import { twMerge } from "../../utils";

import type React from "react";

type CarouselProps = {
  children: React.ReactNode[];
  infinite?: boolean;
  autoPlayInterval?: number;
  draggable?: boolean;
  className?: string;
};

const variants = {
  enter: (direction: number) => ({
    x: direction > 0 ? 300 : -300,
    opacity: 0,
  }),
  center: {
    x: 0,
    opacity: 1,
  },
  exit: (direction: number) => ({
    x: direction > 0 ? -300 : 300,
    opacity: 0,
  }),
};

export const Carousel: React.FC<CarouselProps> = ({
  children,
  infinite = true,
  autoPlayInterval = 0,
  draggable = true,
  className,
}) => {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [direction, setDirection] = useState(0);
  const total = children.length;

  useEffect(() => {
    if (autoPlayInterval > 0) {
      const timer = setInterval(() => {
        nextSlide();
      }, autoPlayInterval);
      return () => clearInterval(timer);
    }
  }, [currentIndex, autoPlayInterval]);

  const nextSlide = () => {
    setDirection(1);
    setCurrentIndex((prev) => (prev === total - 1 ? (infinite ? 0 : prev) : prev + 1));
  };

  const prevSlide = () => {
    setDirection(-1);
    setCurrentIndex((prev) => (prev === 0 ? (infinite ? total - 1 : prev) : prev - 1));
  };

  const goToSlide = (index: number) => {
    setCurrentIndex(index);
  };

  const handleDragEnd = (
    event: PointerEvent | MouseEvent | TouchEvent,
    info: { offset: { x: number; y: number } },
  ) => {
    const dragOffset = info.offset.x;

    if (dragOffset > 50) {
      prevSlide();
    } else if (dragOffset < -50) {
      nextSlide();
    }
  };

  return (
    <div
      className={twMerge(
        "relative flex flex-col items-center justify-center gap-6 overflow-hidden",
        className,
      )}
    >
      <AnimatePresence initial={false} mode="wait" custom={direction}>
        <motion.div
          key={currentIndex}
          custom={direction}
          variants={variants}
          initial="enter"
          animate="center"
          exit="exit"
          transition={{ duration: 0.3 }}
          className="w-full h-full flex items-center justify-center"
          drag={draggable ? "x" : false}
          dragConstraints={{ left: 0, right: 0 }}
          onDragEnd={handleDragEnd}
        >
          {children[currentIndex]}
        </motion.div>
      </AnimatePresence>

      <div className="w-full max-w-[18rem] flex items-center justify-center lg:justify-between gap-3">
        <IconChevronLeftCarousell
          onClick={prevSlide}
          className="hidden lg:block w-[20px] h-[20px] text-blue-500 cursor-pointer"
        />

        <div className="flex space-x-2">
          {children.map((_, idx) => (
            <div
              key={`idx-${idx + 1}`}
              onClick={() => goToSlide(idx)}
              className={twMerge(
                "w-[10px] h-[10px] rounded-full cursor-pointer transition-colors",
                idx === currentIndex ? "bg-blue-500" : "bg-blue-200",
              )}
            />
          ))}
        </div>

        <IconChevronRight
          onClick={nextSlide}
          className="hidden lg:block w-[20px] h-[20px] text-blue-500 cursor-pointer"
        />
      </div>
    </div>
  );
};

export default Carousel;
