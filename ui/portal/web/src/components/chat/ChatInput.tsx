import { Button, Input } from "@left-curve/applets-kit";
import { useState } from "react";

type ChatInputProps = {
  handler: (input: string) => Promise<void>;
  isLoading: boolean;
};

export const ChatInput: React.FC<ChatInputProps> = ({ handler, isLoading }) => {
  const [input, setUserInput] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const prompt = input.trim();
    if (!prompt || isLoading) return;

    setUserInput("");
    await handler(input);
  };

  return (
    <form onSubmit={handleSubmit} className="mt-4 flex items-center">
      <Input
        type="text"
        value={input}
        onChange={(e) => setUserInput(e.target.value)}
        placeholder={isLoading ? "Waiting for response..." : "Sending Message..."}
        disabled={isLoading}
      />
      <Button type="submit" disabled={isLoading}>
        Send
      </Button>
    </form>
  );
};
