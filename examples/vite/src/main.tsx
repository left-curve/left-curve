import { createRoot } from "react-dom/client";
import { AppProvider } from "./providers";

import "@leftcurve/react/fonts/ABCDiatypeRounded/index.css"
import "./index.css";

import App from "./App.tsx";

createRoot(document.getElementById("root")!).render(
  <AppProvider>
    <App />
  </AppProvider>,
);
