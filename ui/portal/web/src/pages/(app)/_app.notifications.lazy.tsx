import { createLazyFileRoute } from "@tanstack/react-router";

import { IconButton, IconChevronDown, twMerge, useMediaQuery } from "@left-curve/applets-kit";
import { Notifications } from "~/components/notifications/Notifications";

export const Route = createLazyFileRoute("/(app)/_app/notifications")({
  component: NotificationApplet,
});

function NotificationApplet() {
  const { isMd } = useMediaQuery();
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
      <div className="flex items-center justify-between">
        <h2 className="flex gap-2 items-center">
          {isMd ? null : (
            <IconButton variant="link" onClick={() => history.go(-1)}>
              <IconChevronDown className="rotate-90" />
            </IconButton>
          )}
          <span className="h3-bold">Notifcations</span>
        </h2>
      </div>
      <Notifications
        maxNotifications={isMd ? 10 : 5}
        withPagination
        className="!shadow-card-shadow"
      />
    </div>
  );
}
