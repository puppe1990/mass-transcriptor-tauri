import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Match original theme bootstrap (dark default, optional light)
const stored = localStorage.getItem("mt-theme");
document.documentElement.setAttribute(
  "data-theme",
  stored === "light" || stored === "dark" ? stored : "dark",
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
