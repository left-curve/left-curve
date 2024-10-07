import { Suspense } from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import { AppProvider } from "./AppProvider";
import { AppRouter } from "./AppRouter";
import { Layout } from "./components/Layout";

import "../public/global.css";
import "@dango/shared/fonts/ABCDiatypeRounded/index.css";

export const App: React.FC = () => {
  return (
    <BrowserRouter>
      <AppProvider>
        <Layout>
          <Suspense fallback={<div>Loading...</div>}>
            <AppRouter />
          </Suspense>
        </Layout>
      </AppProvider>
    </BrowserRouter>
  );
};

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
