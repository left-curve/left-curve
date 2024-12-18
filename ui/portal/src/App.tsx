import ReactDOM from "react-dom/client";
import { Toaster } from "react-hot-toast";

import { AppProvider } from "./AppProvider";
import { AppRouter } from "./AppRouter";

import "../public/global.css";
import "@dango/config/fonts/ABCDiatypeRounded/index.css";
import "@dango/config/fonts/Exposure/index.css";

export const App: React.FC = () => {
  return (
    <AppProvider>
      <AppRouter />
      <Toaster position="bottom-center" />
    </AppProvider>
  );
};

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
