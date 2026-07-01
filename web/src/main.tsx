import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./i18n";
import { applyTheme, resolveInitialTheme } from "./theme";
import "./styles/theme.css";
import "./styles/global.css";
import "./styles/setup.css";
import "./styles/admin.css";

applyTheme(resolveInitialTheme());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
