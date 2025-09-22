import { createLazyFileRoute } from "@tanstack/react-router";

import { MobileTitle } from "~/components/foundation/MobileTitle";
import { ChatBot } from "~/components/chat/ChatBot";

export const Route = createLazyFileRoute("/(app)/_app/chat")({
  component: ChatApplet,
});

function ChatApplet() {
  return (
    <div className="w-full flex flex-col items-center">
      <MobileTitle title="Dango Assistant" className="p-4 pb-0" />
      <ChatBot />
    </div>
  );
}
