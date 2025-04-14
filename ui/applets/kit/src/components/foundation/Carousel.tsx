import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import { IconChevronLeftCarousel, IconChevronRight } from "../";
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

  const changeSlide = (newDirection: number) => {
    if (total <= 1) return;

    setDirection(newDirection);

    if (newDirection > 0) {
      setCurrentIndex((prev) => (prev === total - 1 ? (infinite ? 0 : prev) : prev + 1));
    } else {
      setCurrentIndex((prev) => (prev === 0 ? (infinite ? total - 1 : prev) : prev - 1));
    }
  };

  const nextSlide = () => changeSlide(1);
  const prevSlide = () => changeSlide(-1);

  const goToSlide = (index: number) => {
    if (index === currentIndex) return;
    const newDirection = index > currentIndex ? 1 : -1;
    setDirection(newDirection);
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
        "relative flex flex-col items-center justify-center overflow-hidden",
        className,
      )}
    >
      <div className="relative w-full h-full flex-1">
        <AnimatePresence initial={false} mode="wait" custom={direction}>
          <motion.div
            key={currentIndex}
            custom={direction}
            variants={variants}
            initial="enter"
            animate="center"
            exit="exit"
            transition={{ duration: 0.3 }}
            className="absolute top-0 left-0 w-full h-full flex items-center justify-center"
            drag={draggable ? "x" : false}
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.1}
            onDragEnd={handleDragEnd}
          >
            {children[currentIndex]}
          </motion.div>
        </AnimatePresence>
      </div>

      <div className="w-full max-w-[18rem] flex items-center justify-center lg:justify-between gap-3">
        <IconChevronLeftCarousel
          onClick={prevSlide}
          className="hidden lg:block w-[20px] h-[20px] text-blue-500 cursor-pointer"
        />

        <div className="flex space-x-2">
          {children.map((_, idx) => (
            <button
              type="button"
              key={`idx-${idx + 1}`}
              onClick={() => goToSlide(idx)}
              className={twMerge(
                "w-[10px] h-[10px] rounded-full cursor-pointer transition-colors duration-200 ease-in-out",
                idx === currentIndex ? "bg-blue-500" : "bg-blue-200 hover:bg-blue-300",
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
