import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(auth)/_auth/signup")({
  component: SignupComponent,
});

function SignupComponent() {
  return <div />;
}
