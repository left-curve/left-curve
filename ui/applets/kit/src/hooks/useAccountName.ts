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

export type UseUsernamesReturnType = {
  usernames: Username[];
  info: UsernamesInfo;
  removeUsername: (name: string) => void;
  addUsername: (name: string) => void;
};

export function useUsernames(): UseUsernamesReturnType {
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

  const addUsername = (username: string) => {
    setUsernameInfo((usernames) => {
      console.log(usernames);
      const userInfo = usernames[username] || {};
      const newUserInfo = {
        ...userInfo,
        lastLogin: new Date(),
      };
      return {
        ...usernames,
        [username]: newUserInfo,
      };
    });
  };

  return {
    usernames: Object.keys(usernameInfo),
    info: usernameInfo,
    removeUsername,
    addUsername,
  };
}
