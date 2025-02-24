import type React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "react-hot-toast";
import { Modal } from "./components/Modal";

import { AppProvider } from "./app.provider";
import { AppRouter } from "./app.router";

import "../public/global.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/index.css";
import "@left-curve/ui-config/fonts/Exposure/index.css";

export const App: React.FC = () => {
  return (
    <AppProvider>
      <AppRouter />
      <Toaster position="bottom-center" />
      <Modal />
    </AppProvider>
  );
};

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
