import { IconEmptyStar, IconStar, twMerge } from "@left-curve/applets-kit";
import { motion } from "framer-motion";
import type React from "react";

const containerVariants = {
  hidden: {},
  visible: {
    transition: {
      delayChildren: 0.2,
      staggerChildren: 0.1,
    },
  },
};

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

const appletExampleList = [
  {
    title: "Send & Receive",
    description: "Exchange assets held in your account",
    img: "/images/applets/send-and-receive.svg",
    isFav: true,
  },
  {
    title: "Swap",
    description: "Exchange assets held in your account",
    img: "/images/applets/swap.svg",
  },
  {
    title: "Multisign",
    description: "Description",
    img: "/images/applets/multisig.svg",
  },
  {
    title: "Earn",
    description: "Description",
    img: "/images/applets/earn.svg",
  },
  {
    title: "Block Explorer",
    description: "Description",
    img: "/images/applets/block-explorer.svg",
  },
];

const tokenExample = [
  {
    name: "",
  },
];

export const SearchMenuBody: React.FC = () => {
  return (
    <>
      <motion.div
        className="p-1 w-full flex items-center flex-col gap-1"
        variants={containerVariants}
        initial="hidden"
        animate="visible"
      >
        {appletExampleList.map((applet) => (
          <AppletItem key={applet.title} applet={applet} />
        ))}
      </motion.div>
    </>
  );
};

const AppletItem: React.FC<{
  applet: {
    title: string;
    description: string;
    img: string;
    isFav?: boolean;
  };
}> = ({ applet }) => {
  const { description, img, title, isFav } = applet;
  return (
    <motion.div
      className="w-full p-2 flex items-center justify-between hover:bg-rice-50 rounded-xs"
      variants={childVariants}
      key={title}
    >
      <div className="flex items-center gap-4">
        <div className="p-1 bg-[#FDF0F0] rounded-xxs border border-red-bean-100">
          <img src={img} alt={title} className="w-12 h-12" />
        </div>
        <div>
          <p className="diatype-lg-medium">{title}</p>
          <p className="diatype-m-regular text-gray-500">{description}</p>
        </div>
      </div>
      <div>
        {isFav ? (
          <IconStar className="w-6 h-6 text-rice-500" />
        ) : (
          <IconEmptyStar className="w-6 h-6" />
        )}
      </div>
    </motion.div>
  );
};
