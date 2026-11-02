import React, { Suspense } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { StartupCapsule } from "./components/island/StartupCapsule";
import "./styles/globals.css";

const IslandApp = React.lazy(() =>
  import("./components/IslandApp").then((module) => ({ default: module.IslandApp }))
);
const SettingsApp = React.lazy(() =>
  import("./components/SettingsApp").then((module) => ({ default: module.SettingsApp }))
);

const Root: React.FC = () => {
  const isSettings =
    window.location.search.includes("window=settings") ||
    getCurrentWebviewWindow().label === "settings";

  return (
    <Suspense
      fallback={
        isSettings ? (
          <div className="h-full w-full bg-slate-950" aria-label="Loading ORBIT Settings" />
        ) : (
          <StartupCapsule />
        )
      }
    >
      {isSettings ? <SettingsApp /> : <IslandApp />}
    </Suspense>
  );
};

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>
);
