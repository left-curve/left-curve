import type React from "react";
import { twMerge } from "~/utils";

interface AvatarProps {
  images: string[];
  className?: string;
}

export const AvatarStack: React.FC<AvatarProps> = ({ images, className }) => {
  return (
    <div className={twMerge("flex items-center justify-center h-auto w-max", className)}>
      {images.map((e, i) => {
        return (
          <span
            key={`img-${e}`}
            className="flex relative justify-center items-center z-0 w-8 h-8 rounded-full first:ml-0 -ml-3 overflow-hidden"
          >
            <img
              src={e}
              className="flex object-cover w-full h-full transition-opacity !duration-500 opacity-0 data-[loaded=true]:opacity-100"
              alt={`avatar-${i}`}
              data-loaded="true"
            />
          </span>
        );
      })}
    </div>
  );
};
