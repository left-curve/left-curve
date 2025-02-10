import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/(auth)/_auth/login")({
  component: LoginComponent,
});

function LoginComponent() {
  return <div />;
}
