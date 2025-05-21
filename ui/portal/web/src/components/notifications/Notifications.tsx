import { useNotifications } from "~/hooks/useNotifications";
import { useState } from "react";

import {
  Pagination,
  ResizerContainer,
  twMerge,
  useMediaQuery,
  useWatchEffect,
} from "@left-curve/applets-kit";
import { capitalize } from "@left-curve/dango/utils";

import { Notification } from "./Notification";

import { m } from "~/paraglide/messages";

import type React from "react";
import type { NotificationProps } from "./Notification";
import { AnimatePresence, motion } from "framer-motion";

type NotificationsProps = {
  className?: string;
  maxNotifications?: number;
  withPagination?: boolean;
};

export const Notifications: React.FC<NotificationsProps> = (props) => {
  const { className, maxNotifications = 5, withPagination } = props;
  const [currentPage, setCurrentPage] = useState(1);
  const { isMd } = useMediaQuery();

  const { notifications, hasNotifications, totalNotifications } = useNotifications({
    limit: maxNotifications,
    page: currentPage,
  });

  useWatchEffect(maxNotifications, () => setCurrentPage(1));

  if (!hasNotifications) {
    return (
      <div className="min-h-[19rem] flex flex-col gap-4 items-center justify-center px-4 py-6 text-center relative bg-[url('./images/notifications/bubble-bg.svg')] bg-[-11rem_4rem] bg-no-repeat rounded-xl ">
        <img
          src="/images/notifications/no-notifications.svg"
          alt="no-notifications"
          className="h-[154px]"
        />
        <p className="exposure-m-italic">{m["notifications.noNotifications.title"]()}</p>
        <p className="diatype-m-bold text-gray-500">
          {m["notifications.noNotifications.description"]()}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {hasNotifications && withPagination ? (
        <Pagination
          currentPage={currentPage}
          totalPages={Math.ceil(totalNotifications / maxNotifications)}
          onPageChange={setCurrentPage}
          variant={isMd ? "default" : "text"}
          labelPage={m["pagination.page"]()}
          labelOf={m["pagination.of"]()}
        />
      ) : null}
      <ResizerContainer
        layoutId="notifications"
        className={twMerge("bg-transparent py-1 px-1 rounded-xl shadow-lg", className)}
      >
        <AnimatePresence key={currentPage} mode="wait">
          {Object.entries(notifications).map(([dateKey, n]) => (
            <motion.div
              key={dateKey}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2 }}
            >
              <p className="text-sm text-gray-500 mx-2">{dateKey}</p>
              <div className="flex flex-col gap-2">
                {n.map((notification) => {
                  const NotificationCard = Notification[
                    capitalize(notification.type) as keyof typeof Notification
                  ] as React.FC<NotificationProps>;
                  return (
                    <NotificationCard key={notification.createdAt} notification={notification} />
                  );
                })}
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </ResizerContainer>
    </div>
  );
};
