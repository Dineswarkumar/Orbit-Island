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
    <main className="relative min-h-screen overflow-hidden bg-[#090d16] text-white">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_42%,rgba(14,116,144,0.18),transparent_52%)]" />
      <div className="relative mx-auto flex min-h-screen max-w-6xl flex-col items-center justify-center gap-12 px-6 py-16">
        <header className="text-center">
          <p className="mb-3 text-xs font-semibold tracking-[0.34em] text-cyan-300">ORBIT / LIVE COMPONENT</p>
          <h1 className="text-3xl font-semibold tracking-tight text-slate-100 sm:text-4xl">
            A small island, in motion.
          </h1>
          <p className="mt-3 text-sm text-slate-400">The real compact capsule, rendered with browser-safe demo fixtures.</p>
        </header>

        <section className="flex min-h-64 w-full items-center justify-center rounded-[2rem] border border-white/[0.07] bg-white/[0.025] px-8 py-16 shadow-[0_30px_100px_rgba(0,0,0,0.35)]">
          <div className="theme-obsidian transparency-glass orbit-font-inter">
            <CompactCapsule {...fixtureProps} />
          </div>
        </section>

        <p className="text-xs text-slate-500">
          Demo state: {notificationSnapshotFixture.notifications.length} notifications · {privacyFixture.microphoneApp} using the microphone
        </p>
      </div>
    </main>
  );
}
