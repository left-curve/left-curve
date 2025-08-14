import JsonView from "@microlink/react-json-view";

import type React from "react";

type JsonVisualizerProps = {
  json: string;
  collapsed?: boolean | number;
};

export const JsonVisualizer: React.FC<JsonVisualizerProps> = ({ json, collapsed }) => {
  return (
    <JsonView
      name={false}
      src={JSON.parse(json)}
      displayObjectSize={false}
      displayDataTypes={false}
      collapsed={collapsed}
      indentWidth={1}
      theme={{
        base00: "transparent",
        base01: "var(--color-primary-900)",
        base02: "transparent",
        base03: "var(--color-primary-900)",
        base04: "var(--color-primary-900)",
        base05: "var(--color-primary-900)",
        base06: "var(--color-primary-900)",
        base07: "var(--color-primary-900)", // Keys
        base08: "var(--color-primary-900)",
        base09: "var(--color-primary-900)", // Values
        base0A: "var(--color-primary-900)",
        base0B: "var(--color-primary-900)",
        base0C: "var(--color-primary-900)",
        base0D: "var(--color-foreground-primary-rice)", // Arrow Open
        base0E: "var(--color-foreground-primary-rice)", // Arrow Close
        base0F: "var(--color-primary-900)",
      }}
    />
  );
};
