import { type AppletMetadata, IconEmptyStar, IconStar } from "@left-curve/applets-kit";
import type { AnyCoin, WithPrice } from "@left-curve/dango/types";
import { motion } from "framer-motion";

SearchItem.Applet = Applet;
SearchItem.Asset = Asset;

export { SearchItem };

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

function SearchItem() {
  return null;
}

export function Applet({ description, img, title }: AppletMetadata) {
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
}

export function Asset({ logoURI, name, symbol, price }: WithPrice<AnyCoin>) {
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
}
