import { Suspense } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import { AppProvider } from "./AppProvider";
import { AppRouter } from "./AppRouter";

import "../public/global.css";
import "@dango/shared/fonts/ABCDiatypeRounded/index.css";

export const App: React.FC = () => {
  return (
    <BrowserRouter>
      <AppProvider>
        <AppRouter />
      </AppProvider>
    </BrowserRouter>
  );
};

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
