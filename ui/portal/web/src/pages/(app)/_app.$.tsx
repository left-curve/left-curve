import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(app)/_app/$")({
  component: function NotFound() {
    return (
      <div className="w-full flex flex-1 justify-center items-center p-4">
        <h3 className="text-center max-w-4xl typography-display-xs md:typography-display-xl">
          Sorry, we couldn't find the page you were looking for.
        </h3>
      </div>
    );
  },
});
