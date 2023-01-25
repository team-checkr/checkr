import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { init_hook } from "verification-lawyer";
import "./index.css";

init_hook();
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
