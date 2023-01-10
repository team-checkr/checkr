import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import VerificationLawyer from "verification-lawyer";
import "./index.css";

VerificationLawyer().then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
