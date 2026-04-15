import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import TransformModal from "./TransformModal";
import "./styles.css";

const label = getCurrentWindow().label;

if (label === "transform") {
  document.documentElement.classList.add("transform-html");
  document.body.classList.add("transform-body");
  const w = window as unknown as { __POLISHD_THEME__?: string };
  if (w.__POLISHD_THEME__ === "light" || w.__POLISHD_THEME__ === "dark") {
    document.documentElement.setAttribute("data-theme", w.__POLISHD_THEME__);
  }
}

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
if (label === "transform") {
  root.render(<TransformModal />);
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
