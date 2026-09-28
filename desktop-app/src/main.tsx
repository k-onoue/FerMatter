import { createRoot } from "react-dom/client";
import App from "./App";
import { RecoveryBoundary } from "./RecoveryBoundary";
import InstantWindow from "./InstantWindow";
import "./style.css";
createRoot(document.getElementById("root")!).render(
  <RecoveryBoundary>
    {location.hash.startsWith("#instant") ? <InstantWindow /> : <App />}
  </RecoveryBoundary>,
);
