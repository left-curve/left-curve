import ReactDOM from "react-dom/client";
import { App } from "./app";

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(<App />);
