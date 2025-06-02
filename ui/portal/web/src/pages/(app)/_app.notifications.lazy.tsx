import { useMediaQuery } from "@left-curve/applets-kit";

import { Notifications } from "~/components/notifications/Notifications";
import { MobileTitle } from "~/components/foundation/MobileTitle";

import { createLazyFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/notifications")({
  component: NotificationApplet,
});

function NotificationApplet() {
  const { isMd } = useMediaQuery();
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
      <MobileTitle title={m["notifications.title"]()} />
      <Notifications
        maxNotifications={isMd ? 10 : 5}
        withPagination
        className="!shadow-account-card bg-rice-25"
      />
    </div>
  );
}
