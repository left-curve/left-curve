import { type AppletMetadata, IconEmptyStar, IconStar } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import type React from "react";

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

export const AppletItem: React.FC<AppletMetadata> = ({ description, img, title }) => {
  return (
    <motion.div
      className="w-full p-2 flex items-center justify-between hover:bg-rice-50 rounded-xs ] group-data-[selected=true]:bg-rice-50"
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
        {/*  {false ? (
          <IconStar className="w-6 h-6 text-rice-500" />
        ) : (
          <IconEmptyStar className="w-6 h-6" />
        )} */}
      </div>
    </motion.div>
  );
};
