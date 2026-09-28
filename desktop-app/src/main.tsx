import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import InstantWindow from "./InstantWindow";
import "./style.css";
createRoot(document.getElementById("root")!).render(
  location.hash.startsWith("#instant") ? <InstantWindow /> : <App />,
);
