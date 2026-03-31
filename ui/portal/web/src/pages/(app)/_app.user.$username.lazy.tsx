import { createLazyFileRoute } from "@tanstack/react-router";
import { UserExplorer } from "~/components/explorer/UserExplorer";
import { MobileTitle } from "~/components/foundation/MobileTitle";

export const Route = createLazyFileRoute("/(app)/_app/user/$username")({
  component: UserExplorerApplet,
});

function UserExplorerApplet() {
  const { username } = Route.useParams();

  return (
    <div className="w-full flex flex-col items-center">
      <MobileTitle title="User Profile" className="p-4 pb-0" />
      <UserExplorer username={username}>
        <UserExplorer.NotFound />
        <UserExplorer.Header />
        <UserExplorer.Content />
      </UserExplorer>
    </div>
  );
}
