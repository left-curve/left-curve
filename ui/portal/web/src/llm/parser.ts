import type { LLMessage } from "./model";

export class StreamParser {
  private messages: LLMessage[] = [];
  private state: "text" | "think" | "function" = "text";
  private buffer = "";

  private get lastMessage(): LLMessage | undefined {
    return this.messages[this.messages.length - 1];
  }

  private addText(content: string) {
    if (!content) return;
    if (this.lastMessage?.type === "text") {
      this.lastMessage.content += content;
    } else {
      this.messages.push({ type: "text", content, author: "assistant" });
    }
  }

  feed(chunk: string): LLMessage[] {
    this.buffer += chunk;

    while (true) {
      const initialBufferLength = this.buffer.length;

      if (this.state === "text") {
        const tagStartIndex = this.buffer.indexOf("<");

        if (tagStartIndex === -1) {
          this.addText(this.buffer);
          this.buffer = "";
          break;
        }

        const textContent = this.buffer.substring(0, tagStartIndex);
        this.addText(textContent);
        this.buffer = this.buffer.substring(tagStartIndex);

        const tagEndIndex = this.buffer.indexOf(">");
        if (tagEndIndex === -1) {
          break;
        }

        const tagName = this.buffer.substring(1, tagEndIndex);
        if (tagName === "think") {
          this.state = "think";
          this.messages.push({
            type: "thinking",
            content: "",
            author: "assistant",
          });
        } else if (tagName === "function") {
          this.state = "function";
          this.messages.push({
            type: "function",
            content: "",
            author: "assistant",
          });
        }

        this.buffer = this.buffer.substring(tagEndIndex + 1);
      } else {
        const endTagStartIndex = this.buffer.indexOf("<");

        if (endTagStartIndex !== -1) {
          const content = this.buffer.substring(0, endTagStartIndex);
          if (
            this.lastMessage &&
            (this.lastMessage.type === "thinking" || this.lastMessage.type === "function")
          ) {
            this.lastMessage.content += content;
          }
          this.buffer = this.buffer.substring(endTagStartIndex);

          const endTagEndIndex = this.buffer.indexOf(">");
          if (endTagEndIndex !== -1) {
            const closingTag = this.buffer.substring(0, endTagEndIndex + 1);
            const expectedTag = `</${this.state}>`;

            if (closingTag === expectedTag) {
              this.buffer = this.buffer.substring(endTagEndIndex + 1);
              this.state = "text";
            }
          } else {
            break;
          }
        } else {
          if (
            this.lastMessage &&
            (this.lastMessage.type === "thinking" || this.lastMessage.type === "function")
          ) {
            this.lastMessage.content += this.buffer;
          }
          this.buffer = "";
          break;
        }
      }

      if (this.buffer.length > 0 && this.buffer.length === initialBufferLength) {
        break;
      }
    }

    return [...this.messages];
  }
}
