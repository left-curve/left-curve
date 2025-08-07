import { createFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

import { Landing } from "~/components/landing/Landing";

export function getFullpageLicenseKey() {
  if (!process.env.NEXT_PUBLIC_FULLPAGE_KEY) return "FALLBACK_KEY";
  return new TextDecoder("utf-8", { fatal: true }).decode(
    Uint8Array.from(atob(process.env.NEXT_PUBLIC_FULLPAGE_KEY), (c) => c.charCodeAt(0)),
  );
}

export const Route = createFileRoute("/(app)/_app/")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.overview"]()}` }],
  }),
  component: OverviewComponent,
});

function OverviewComponent() {
  return (
    <Landing>
      <Landing.Header />
      <Landing.SectionRice />
      <Landing.SectionRed />
      <Landing.SectionGreen />
    </Landing>
  );
}
