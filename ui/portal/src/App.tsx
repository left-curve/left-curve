import ReactDOM from "react-dom/client";
import { Toaster } from "react-hot-toast";
import { BrowserRouter } from "react-router-dom";

import { AppProvider } from "./AppProvider";
import { AppRouter } from "./AppRouter";

import "../public/global.css";
import "@dango/config/fonts/ABCDiatypeRounded/index.css";
import "@dango/config/fonts/Exposure/index.css";

export const App: React.FC = () => {
  return (
    <BrowserRouter>
      <AppProvider>
        <AppRouter />
        <Toaster position="bottom-center" />
      </AppProvider>
    </BrowserRouter>
  );
};

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
