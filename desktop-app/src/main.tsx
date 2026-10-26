import { createRoot } from "react-dom/client";
import App from "./App";
import { RecoveryBoundary } from "./RecoveryBoundary";
import InstantWindow from "./InstantWindow";
import { logError } from "./log";
import { isWindows } from "./platform";
import "./style.css";
if (isWindows()) document.documentElement.dataset.platform = "windows";
window.addEventListener("error", (event) =>
  logError("window.error", event.error ?? event.message),
);
window.addEventListener("unhandledrejection", (event) =>
  logError("unhandledrejection", event.reason),
);
createRoot(document.getElementById("root")!).render(
  <RecoveryBoundary>
    {location.hash.startsWith("#instant") ? <InstantWindow /> : <App />}
  </RecoveryBoundary>,
);
