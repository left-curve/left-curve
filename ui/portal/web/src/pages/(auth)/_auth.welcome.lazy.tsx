import { Button } from "@left-curve/applets-kit";
import { Link, createLazyFileRoute } from "@tanstack/react-router";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createLazyFileRoute("/(auth)/_auth/welcome")({
  component: WelcomeComponent,
});

function WelcomeComponent() {
  return (
    <div className="h-screen w-screen flex items-center justify-center flex-col-reverse gap-10 lg:gap-60 lg:flex-row p-4">
      <div className="bg-[url('./images/waves.svg')] w-full h-[3rem] bg-cover bg-no-repeat bg-bottom fixed top-0 left-0" />
      <div className="w-full flex flex-col gap-10 max-w-[366px]">
        <img
          src="./favicon.svg"
          alt="dango-logo"
          className="h-12 rounded-full shadow-account-card"
        />
        <div className="flex flex-col gap-6 items-center">
          <div className="flex flex-col gap-2 items-center justify-center text-center">
            <h1 className="display-heading-xl">{m["welcome.title"]()}</h1>
            <p className="text-tertiary-500 diatype-lg-regular">{m["welcome.description"]()}</p>
          </div>
          <Button as={Link} className="w-full md:w-[260px]" to="/">
            {m["common.continue"]()}
          </Button>
        </div>
      </div>
      <img
        className="max-w-[313px] lg:max-w-[449px] w-full h-auto"
        src="./images/welcome.svg"
        alt="welcome-image"
      />
    </div>
  );
}
