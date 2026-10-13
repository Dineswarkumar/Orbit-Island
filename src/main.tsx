import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { IslandApp } from "./components/IslandApp";
import { SettingsApp } from "./components/SettingsApp";
import "./styles/globals.css";

const Root: React.FC = () => {
  const isSettings =
    window.location.search.includes("window=settings") ||
    getCurrentWebviewWindow().label === "settings";

  return isSettings ? <SettingsApp /> : <IslandApp />;
};

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>
);
