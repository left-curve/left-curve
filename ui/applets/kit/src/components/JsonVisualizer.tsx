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
        base00: "#453d39",
        base01: "#fffcf6",
        base02: "#453d39",
        base03: "#fffcf6",
        base04: "#fffcf6",
        base05: "#fffcf6",
        base06: "#fffcf6",
        base07: "#fff3e1", // Keys
        base08: "#fffcf6",
        base09: "#fffcf6", // Values
        base0A: "#fffcf6",
        base0B: "#fffcf6",
        base0C: "#fffcf6",
        base0D: "#d4882c", // Arrow Open
        base0E: "#d4882c", // Arrow Close
        base0F: "#fffcf6",
      }}
    />
  );
};
