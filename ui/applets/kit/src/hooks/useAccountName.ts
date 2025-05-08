import { useStorage } from "@left-curve/store";
import { useCallback } from "react";

import type { Username } from "@left-curve/dango/types";
type UsernamesInfo = {
  [username: string]: {
    lastLogin?: Date;
    accounts?: {
      [address: string]: {
        name: string;
      };
    };
  };
};

export type UseUserNameReturnType = {
  usernames: Username[];
  info: UsernamesInfo;
  removeUsername: (name: string) => void;
  addUsername: (name: string) => void;
};

export function useUsernames(): UseUserNameReturnType {
  const [usernameInfo, setUsernameInfo] = useStorage<UsernamesInfo>("app.usernames", {
    initialValue: {},
  });

  const removeUsername = useCallback((username: string) => {
    setUsernameInfo((prevUsernames) => {
      const newUsernames = { ...prevUsernames };
      delete newUsernames[username];
      return newUsernames;
    });
  }, []);

  const addUsername = useCallback((username: string) => {
    setUsernameInfo((prevUsernames) => {
      const userExists = !!prevUsernames[username];
      const updatedUser = {
        lastLogin: new Date(),
        accounts: userExists ? prevUsernames[username].accounts : {},
      };
      return {
        ...prevUsernames,
        [username]: updatedUser,
      };
    });
  }, []);

  return {
    usernames: Object.keys(usernameInfo),
    info: usernameInfo,
    removeUsername,
    addUsername,
  };
}
