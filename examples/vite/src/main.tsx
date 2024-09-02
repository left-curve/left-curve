import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { GrugProvider } from "@leftcurve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "./configs/connect-kit";

import { ModalRoot } from "@leftcurve/react/components";

import "./index.css";

import App from "./App.tsx";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <GrugProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <App />
        <ModalRoot />
      </QueryClientProvider>
    </GrugProvider>
  </StrictMode>,
);
