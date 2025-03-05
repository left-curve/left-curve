import { motion } from "framer-motion";

import type { AnyCoin, WithPrice } from "@left-curve/dango/types";
import type React from "react";

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

export const AssetItem: React.FC<WithPrice<AnyCoin>> = ({ logoURI, name, symbol, price }) => {
  return (
    <motion.div
      className="w-full p-2 min-h-[74px] flex items-start justify-between hover:bg-rice-50 rounded-xs group-data-[selected=true]:bg-rice-50"
      variants={childVariants}
      key={name}
    >
      <div className="flex items-start gap-4">
        <img src={logoURI} alt={name} className="w-8 h-8 rounded-full" />
        <div className="flex flex-col gap-1">
          <p className="diatype-m-bold">{name}</p>
          <p className="diatype-m-regular text-gray-500">{symbol}</p>
          {/* <p className="diatype-m-regular text-gray-500">{link}</p> */}
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <p className="diatype-sm-bold">${price}</p>
      </div>
    </motion.div>
  );
};
