import { createLazyFileRoute } from "@tanstack/react-router";

import { MobileTitle, useMediaQuery } from "@left-curve/applets-kit";
import { Notifications } from "~/components/notifications/Notifications";

import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/notifications")({
  component: NotificationApplet,
});

function NotificationApplet() {
  const { isMd } = useMediaQuery();
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
      <div className="flex items-center justify-between">
        <MobileTitle action={() => history.go(-1)} title={m["notifications.title"]()} />
      </div>
      <Notifications
        maxNotifications={isMd ? 10 : 5}
        withPagination
        className="!shadow-card-shadow bg-rice-25"
      />
    </div>
  );
}
