import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import VerificationLawyer, { init_hook } from "verification-lawyer";
import "./index.css";

VerificationLawyer().then(() => {
  init_hook();
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
