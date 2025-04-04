import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

export const Route = createFileRoute("/(auth)/_auth/signin")({
  validateSearch: z.object({
    socketId: z.string().optional(),
  }),
});
