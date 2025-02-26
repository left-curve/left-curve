import { Button } from "@left-curve/applets-kit";
import { Link, createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(auth)/_auth/welcome")({
  component: WelcomeComponent,
});

function WelcomeComponent() {
  return (
    <div className="h-screen w-screen flex items-center justify-center flex-col-reverse gap-10 lg:gap-60 lg:flex-row p-4">
      <div className="bg-[url('./images/waves.svg')] w-full h-[3rem] bg-cover bg-no-repeat bg-bottom fixed top-0 left-0" />
      <div className="w-full flex flex-col gap-10 max-w-[366px]">
        <img
          src="./favicon.svg rounded-full shadow-btn-shadow-gradient"
          alt="dango-logo"
          className="h-12"
        />
        <div className="flex flex-col gap-6 items-center">
          <div className="flex flex-col gap-2 items-center justify-center text-center">
            <h1 className="display-heading-xl">Welcome to Dango</h1>
            <p className="text-gray-500 diatype-lg-regular">
              We’re delighted to have you with us and look forward to seeing what you do next.
            </p>
          </div>
          <Button as={Link} className="w-full md:w-[260px]" to="/">
            Complete Signup
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
