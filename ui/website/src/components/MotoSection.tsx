import { motion, useScroll, useSpring, useTransform } from "framer-motion";

const MotoSection: React.FC = () => {
  const { scrollYProgress } = useScroll();

  const springProcess = useSpring(scrollYProgress, { stiffness: 300, damping: 50, mass: 0.3 });
  const translateY = useTransform(springProcess, [0, 1], [50, -200]);
  const scale = useTransform(springProcess, [0, 1], [0, 1]);

  return (
    <motion.div
      transition={{ delay: 1 }}
      style={{ translateY }}
      className="flex flex-col gap-8 md:gap-24 items-center px-4 will-change-transform"
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
        className="text-typography-pink-200 drop-shadow-lg text-[28px] leading-[34px] md:text-[80px] md:leading-[96px] italic will-change-[scale]"
        style={{ scale }}
      >
        Coming soon
      </motion.h2>
    </motion.div>
  );
};

export default MotoSection;
