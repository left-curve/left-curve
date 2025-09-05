import { useState } from "react";
import { useNotifications } from "~/hooks/useNotifications";

import { ResizerContainer, Spinner, twMerge, useInfiniteScroll } from "@left-curve/applets-kit";

import { AnimatePresence, motion } from "framer-motion";
import { Notification } from "./Notification";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

type NotificationsProps = {
  className?: string;
  notificationsPerCall?: number;
};

export const Notifications: React.FC<NotificationsProps> = (props) => {
  const { className, notificationsPerCall = 5 } = props;

  const [notificationsVisible, setNotificationsVisible] = useState(notificationsPerCall);
  const { notifications, hasNotifications, totalNotifications } = useNotifications({
    limit: notificationsVisible,
  });

  const hasMoreNotifications = notificationsVisible < totalNotifications;

  const { loadMoreRef } = useInfiniteScroll(() => {
    setNotificationsVisible((prev) => Math.min(prev + notificationsPerCall, totalNotifications));
  }, hasMoreNotifications);

  if (!hasNotifications) {
    return (
      <div className="min-h-[19rem] flex flex-col shadow-account-card gap-4 items-center justify-center px-4 py-6 text-center relative bg-[url('./images/notifications/bubble-bg.svg')] bg-[-11rem_4rem] bg-no-repeat rounded-xl ">
        <img
          src="/images/notifications/no-notifications.svg"
          alt="no-notifications"
          className="h-[154px]"
        />
        <p className="exposure-m-italic">{m["notifications.noNotifications.title"]()}</p>
        <p className="diatype-m-bold text-tertiary-500">
          {m["notifications.noNotifications.description"]()}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <ResizerContainer
        layoutId="notifications"
        className={twMerge("bg-transparent py-1 px-1 rounded-xl", className)}
      >
        <AnimatePresence mode="wait">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            {Object.entries(notifications).map(([dateKey, n]) => (
              <motion.div key={dateKey}>
                <p className="text-sm text-tertiary-500 mx-2 my-1">{dateKey}</p>
                <div className="flex flex-col gap-2 max-w-full">
                  {n.map((notification) => (
                    <Notification key={notification.id} notification={notification} />
                  ))}
                </div>
              </motion.div>
            ))}
            {hasMoreNotifications ? (
              <div ref={loadMoreRef} className="flex justify-center py-2">
                <Spinner color="pink" />
              </div>
            ) : null}
          </motion.div>
        </AnimatePresence>
      </ResizerContainer>
    </div>
  );
};
