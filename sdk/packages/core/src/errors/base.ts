type BaseErrorParameters = {
  cause?: BaseError | Error | undefined;
  details?: string | undefined;
  metaMessages?: string[] | undefined;
  name?: string | undefined;
};

export class BaseError extends Error {
  details: string;
  metaMessages?: string[] | undefined;
  shortMessage: string;

  override name = "BaseError";
  constructor(shortMessage: string, args: BaseErrorParameters = {}) {
    const details = (() => {
      if (args.cause instanceof BaseError) return args.cause.details;
      if (args.cause?.message) return args.cause.message;
      return args.details!;
    })();

    const message = [
      shortMessage || "An error occurred.",
      "",
      ...(args.metaMessages ? [...args.metaMessages, ""] : []),
      ...(details ? [`Details: ${details}`] : []),
    ].join("\n");

    super(message, args.cause ? { cause: args.cause } : undefined);

    this.details = details;
    this.name = args.name ?? this.name;
    this.shortMessage = shortMessage;
    this.metaMessages = args.metaMessages;
  }
}
