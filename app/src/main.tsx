import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/saira/wght.css";
import "@fontsource/barlow/400.css";
import "@fontsource/barlow/500.css";
import "@fontsource/barlow/600.css";
import "./styles.css";
import App from "./App";
import { Crash } from "./Crash";
import { fadeClosedDialogs } from "./dialogFade";
import { currentSettings, updateSettings } from "./settings";

// For tools/perf.mjs: the dev build lets the timing suite switch settings the way the UI does.
if (import.meta.env.DEV) Object.assign(window, { __obsidian: { settings: currentSettings, updateSettings } });

fadeClosedDialogs();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Crash>
      <App />
    </Crash>
  </StrictMode>,
);
