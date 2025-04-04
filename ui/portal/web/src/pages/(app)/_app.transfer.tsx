import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

export const Route = createFileRoute("/(app)/_app/transfer")({
  validateSearch: z.object({
    action: z.enum(["send", "receive"]).catch("send"),
  }),
});
