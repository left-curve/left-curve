import { createLazyFileRoute } from "@tanstack/react-router";

import {
  IconButton,
  IconChevronDown,
  IconGear,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { NotificationsList } from "~/components/notifications/NotificationsList";

export const Route = createLazyFileRoute("/(app)/_app/notifications")({
  component: NotificationComponent,
});

function NotificationComponent() {
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
        <IconGear className="w-[22px] h-[22px] text-rice-700" />
      </div>
      <div className={twMerge("bg-rice-25 w-full shadow-card-shadow rounded-3xl")}>
        <NotificationsList notifications={[]} />
      </div>
    </div>
  );
}
