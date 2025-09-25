/** biome-ignore-all lint/suspicious/noArrayIndexKey: <Iteration with index necessary> */

import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { useAnimationFrame, useMotionValue, useTransform, useReducedMotion } from "framer-motion";

import { motion } from "framer-motion";
import { twMerge } from "@left-curve/foundation";

import type React from "react";

interface MarqueeProps {
  item: string | React.ReactNode;
  className?: string;
  speed?: number;
  direction?: "left" | "right";
}

export const Marquee: React.FC<MarqueeProps> = ({
  item,
  className,
  speed = 80,
  direction = "left",
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const baseRef = useRef<HTMLDivElement>(null);

  const [containerW, setContainerW] = useState(0);
  const [contentW, setContentW] = useState(0);
  const reduced = useReducedMotion();

  useLayoutEffect(() => {
    const c = containerRef.current;
    const b = baseRef.current;
    if (!c || !b) return;

    const measure = () => {
      setContainerW(c.getBoundingClientRect().width);
      setContentW(b.getBoundingClientRect().width);
    };

    measure();
    const roC = new ResizeObserver(measure);
    const roB = new ResizeObserver(measure);
    roC.observe(c);
    roB.observe(b);
    return () => {
      roC.disconnect();
      roB.disconnect();
    };
  }, []);

  const clones = useMemo(() => {
    if (!containerW || !contentW) return 3;
    return Math.max(2, Math.ceil(containerW / contentW) + 2);
  }, [containerW, contentW]);

  const baseX = useMotionValue(0);

  useAnimationFrame((_, delta) => {
    if (reduced || !contentW) return;
    const dir = direction === "left" ? 1 : -1;
    baseX.set(baseX.get() + dir * speed * (delta / 1000));
  });

  const x = useTransform(baseX, (v) => {
    if (!contentW) return 0;
    const m = ((v % contentW) + contentW) % contentW;
    return -m;
  });

  const Content = (
    <div className="inline-flex shrink-0 items-center whitespace-nowrap">
      {typeof item === "string" ? <span>{item}</span> : item}
    </div>
  );

  return (
    <div ref={containerRef} className={twMerge("relative w-full overflow-hidden", className)}>
      <motion.div className="flex items-center" style={{ x, willChange: "transform" }}>
        <div ref={baseRef} className="inline-flex shrink-0 items-center whitespace-nowrap">
          {typeof item === "string" ? <span>{item}</span> : item}
        </div>
        {Array.from({ length: clones - 1 }).map((_, i) => (
          <div key={i} className="inline-flex shrink-0 items-center whitespace-nowrap" aria-hidden>
            {Content}
          </div>
        ))}
      </motion.div>
    </div>
  );
};
