import { useState } from "react";
import type { LLMessage } from "~/llm/model";

type MessageRendererProps = {
  message: LLMessage;
};

export const MessageRenderer: React.FC<MessageRendererProps> = ({ message }) => {
  const isUser = message.author === "user";

  return (
    <div className={`flex my-2 ${isUser ? "justify-end" : "justify-start"}`}>
      {message.type === "text" && <MessageText {...message} />}
      {message.type === "thinking" && <MessageThinking {...message} />}
      {message.type === "function" && <MessageFunction {...message} />}
    </div>
  );
};

const MessageText: React.FC<LLMessage> = ({ content, author }) => {
  const isUser = author === "user";
  return (
    <p
      className={`p-3 rounded-lg max-w-xl shadow ${isUser ? "bg-blue-600 text-white" : "bg-gray-700 text-white"}`}
    >
      {content}
    </p>
  );
};

const MessageThinking: React.FC<LLMessage> = ({ content }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="border border-gray-600 rounded-lg p-2 bg-gray-800 text-gray-400 w-full max-w-xl">
      <button type="button" className="text-sm" onClick={() => setOpen(!open)}>
        {open ? "▼ Process Thinking" : "▶ Process Thinking"}
      </button>
      {open && <pre className="mt-2 text-xs whitespace-pre-wrap text-gray-300">{content}</pre>}
    </div>
  );
};

const MessageFunction: React.FC<LLMessage> = ({ content }) => {
  return (
    <div className="border border-indigo-700 rounded-lg p-2 bg-gray-800 w-full max-w-2xl">
      <div className="text-sm font-bold text-indigo-400 mb-1">⚙️ Tool</div>
      <pre className="bg-gray-900 p-2 rounded text-sm text-gray-300 overflow-auto">{content}</pre>
    </div>
  );
};
