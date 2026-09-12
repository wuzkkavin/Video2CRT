/**
 * React entry point. Mounts the App component into #root.
 *
 * StrictMode is on. App.tsx registers Tauri event listeners in a
 * useEffect AND returns an unlisten cleanup, so the dev-mode
 * mount → unmount → remount cycle is handled correctly — the second
 * mount gets a fresh set of listeners.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";

const rootEl = document.getElementById("root");
if (!rootEl) {
  throw new Error("Root element #root not found in index.html");
}

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
