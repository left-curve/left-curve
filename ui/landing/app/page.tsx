"use client";

import { useMediaQuery } from "@dango/shared";
import {
  MotionValue,
  motion,
  useMotionValue,
  useScroll,
  useSpring,
  useTransform,
} from "framer-motion";
import { useRouter } from "next/navigation";
import React, { useRef } from "react";
import { Footer } from "~/components/Footer";

function HomePage() {
  const targetRef = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll();
  const { push } = useRouter();
  const isMd = useMediaQuery("md");
  const springProcess = useSpring(scrollYProgress, { stiffness: 300, damping: 50, mass: 0.3 });
  const translateY = useTransform(
    springProcess,
    [0, 1],
    isMd ? ["3.25rem", "-12.5rem"] : ["8rem", "-8rem"],
  );

  return (
    <div
      className="flex flex-1 flex-col w-full relative items-center justify-between pb-4 scrollbar-none"
      style={{ minHeight: window.innerHeight * 1.3 }}
      ref={targetRef}
    >
      <div className="fixed mx-0 top-6 p-4 z-50 rounded-2xl">
        <img src="/images/logo.webp" alt="logo" className=" h-6 md:h-12 object-contain" />
      </div>
      <motion.div
        className="header-landing pb-20 pt-[72px] md:pt-32 w-full flex flex-col gap-12 items-center justify-center px-4"
        style={{ height: isMd ? window.innerHeight * 0.95 : window.innerHeight * 0.75 }}
      >
        <motion.picture className="object-contain md:max-w-[80%] w-full max-h-full">
          <motion.source srcSet="/images/background.svg" media="(min-width: 1280px)" />
          <motion.img
            src="/images/background-mobile.svg"
            alt="background-mobile"
            className="w-full max-h-full"
          />
        </motion.picture>
      </motion.div>
      <motion.div
        transition={{ delay: 1 }}
        style={{ translateY }}
        className="flex flex-col gap-8 md:gap-24 items-center px-4"
      >
        <motion.h1 className="text-4xl md:text-7xl font-extrabold max-w-[1030px] italic text-center">
          Bringing back the good things of the last cycle
        </motion.h1>
        {/* <motion.button
          style={{ scale }}
          onClick={() => push("/auth/login")}
          className="text-lg md:text-8xl bg-surface-pink-200 px-8 py-3 md:px-[72px] md:py-4 rounded-[20px] md:rounded-[48px] font-extrabold text-surface-rose-200 italic w-fit"
        >
          Enter Portal
        </motion.button> */}
        <motion.h2
          className="text-typography-pink-200 drop-shadow-lg text-[28px] leading-[34px] md:text-[80px] md:leading-[96px] italic"
          initial={{ scale: 0 }}
          transition={{ duration: 0.7 }}
          whileInView={{ scale: 1 }}
        >
          Coming soon
        </motion.h2>
      </motion.div>
      <Footer className="w-full" />
    </div>
  );
}

export default HomePage;
