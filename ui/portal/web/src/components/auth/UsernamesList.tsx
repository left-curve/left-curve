import { IconButton, IconChevronRight, IconTrash, useUsernames } from "@left-curve/applets-kit";
import { format } from "date-fns";
import type React from "react";
import { Fragment } from "react";
import { m } from "~/paraglide/messages";

interface Props {
  usernames: {
    [username: string]: {
      lastLogin?: Date;
      accounts?: {
        [address: string]: {
          name: string;
        };
      };
    };
  };
  onClick: (username: string) => void;
  showRemove?: boolean;
  showArrow?: boolean;
}

export const UsernamesList: React.FC<Props> = ({ onClick, showRemove, usernames, showArrow }) => {
  const { removeUsername } = useUsernames();
  return (
    <div className="flex flex-col w-full ">
      {Object.keys(usernames).map((username, i) => {
        const { lastLogin } = usernames[username];
        return (
          <Fragment key={username}>
            <div
              className="flex gap-2 w-full hover:bg-rice-50 p-2 rounded-xs transition-all cursor-pointer"
              onClick={() => onClick(username)}
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
                  {lastLogin && (
                    <p className="diatype-m-regular text-gray-500">
                      {m["signin.rememberUsername.lastLogin"]({
                        time: format(lastLogin, "dd/MM/yyyy"),
                      })}
                    </p>
                  )}
                </div>
                {showRemove && (
                  <IconButton
                    variant="link"
                    className="text-gray-500 hover:text-red-bean-400"
                    onClick={(event) => {
                      event.stopPropagation();
                      removeUsername(username);
                    }}
                  >
                    <IconTrash className="w-6 h-6" />
                  </IconButton>
                )}
                {showArrow && (
                  <IconButton
                    variant="link"
                    className="text-gray-500 p-0 h-fit w-fit"
                    onClick={() => onClick(username)}
                  >
                    <IconChevronRight className="w-6 h-6" />
                  </IconButton>
                )}
              </div>
            </div>
            {Object.keys(usernames).length - 1 !== i && (
              <span className="w-full h-[1px] bg-gray-100" />
            )}
          </Fragment>
        );
      })}
    </div>
  );
};
