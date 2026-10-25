import type { ComponentProps } from "react";
import { THEMES } from "@app/core/theme";
import { CompactCapsule } from "@app/components/island/CompactCapsule";
import {
  appConfigFixture,
  audioSpectrumFixture,
  mediaSessionFixture,
  notificationSnapshotFixture,
  privacyFixture,
} from "./mocks/fixtures";
import { IslandStage } from "./stage/IslandStage";

const activeThemeMeta = THEMES.find((theme) => theme.id === appConfigFixture.theme)!;

const fixtureProps: ComponentProps<typeof CompactCapsule> = {
  isExpanded: false,
  prefersReducedMotion: false,
  config: appConfigFixture,
  activeThemeMeta,
  transientNotif: null,
  isNotifEntering: false,
  isNotifHovered: false,
  setIsNotifHovered: () => undefined,
  onViewNotification: () => undefined,
  onDismissNotification: () => undefined,
  isMediaActive: true,
  mediaSession: mediaSessionFixture,
  timeDisplay: { timeStr: "10:42", ampm: "AM" },
  privacyLight: (
    <div className="flex items-center gap-1.5 text-[10px] font-semibold tracking-wide text-emerald-300">
      <span className="h-1.5 w-1.5 rounded-full bg-emerald-300 shadow-[0_0_8px_rgba(110,231,183,0.9)]" />
      MIC
    </div>
  ),
  hardwareMetrics: undefined,
  audioSpectrum: audioSpectrumFixture,
  onOpenHome: () => undefined,
};

export default function App() {
  return (
    <div className="site-shell">
      <div className="site-atmosphere" aria-hidden="true" />
      <IslandStage>
        <CompactCapsule {...fixtureProps} />
      </IslandStage>
      <footer className="site-demo-note">
        Live component preview · {notificationSnapshotFixture.notifications.length} notifications · {privacyFixture.microphoneApp} using the microphone
      </footer>
    </div>
  );
}
