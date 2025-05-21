import { IconButton, IconChevronRight } from "@left-curve/applets-kit";
import { Fragment } from "react";

import type { Username } from "@left-curve/dango/types";
import type React from "react";

type UsernamesListProps = {
  usernames: Username[];
  onUserSelection: (username: string) => void;
};

export const UsernamesList: React.FC<UsernamesListProps> = ({ usernames, onUserSelection }) => {
  return (
    <div className="flex flex-col w-full ">
      {usernames.map((username, i) => {
        return (
          <Fragment key={username}>
            <div
              className="flex gap-2 w-full hover:bg-rice-50 p-2 rounded-xs transition-all cursor-pointer"
              onClick={() => onUserSelection(username)}
            >
              <div className="w-12 h-12 bg-[#FDF0F0] rounded-xxs border border-red-bean-100 flex items-center justify-center">
                <img
                  src="/images/emojis/simple/username.svg"
                  alt="username"
                  className="w-10 h-10"
                />
              </div>
              <div className="flex-1 flex gap-4 items-center justify-between">
                <div className="flex flex-col gap-1">
                  <p className="diatype-lg-medium text-gray-700">{username}</p>
                </div>

                <IconButton variant="link" className="text-gray-500 p-0 h-fit w-fit">
                  <IconChevronRight className="w-6 h-6" />
                </IconButton>
              </div>
            </div>
            {usernames.length - 1 !== i && <span className="w-full h-[1px] bg-gray-100" />}
          </Fragment>
        );
      })}
    </div>
  );
};
