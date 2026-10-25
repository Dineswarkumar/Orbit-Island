import {
  notificationSnapshotFixture,
  privacyFixture,
} from "./mocks/fixtures";
import { IslandStage } from "./stage/IslandStage";

export default function App() {
  return (
    <div className="site-shell">
      <div className="site-atmosphere" aria-hidden="true" />
      <IslandStage />
      <footer className="site-demo-note">
        Live component preview · {notificationSnapshotFixture.notifications.length} notifications · {privacyFixture.microphoneApp} using the microphone
      </footer>
    </div>
  );
}
