import { useState } from "react";
import { THEMES } from "@app/core/theme";
import { CompactCapsule } from "@app/components/island/CompactCapsule";
import { HomeSection } from "@app/components/island/HomeSection";
import { MultiTaskSection } from "@app/components/island/MultiTaskSection";
import { CalendarSection } from "@app/components/island/CalendarSection";
import { HardwareSection } from "@app/components/island/HardwareSection";
import { useAudioSpectrum } from "../audio/analyser";
import {
  appConfigFixture,
  hardwareMetricsFixture,
  mediaSessionFixture,
  notificationFixtures,
  reminderFixtures,
  shelfFixtures,
} from "../mocks/fixtures";

export { useAudioSpectrum as useSyntheticSpectrum };

export function DualPrivacyLight() {
  return (
    <span className="inline-flex items-center space-x-1 shrink-0">
      <span
        className="relative inline-flex items-center justify-center w-4 h-4 rounded-full shrink-0 animate-pulse"
        style={{
          backgroundColor: "#22c55e28",
          border: "1px solid #22c55eb3",
          boxShadow: "0 0 8px #22c55e80",
          color: "#22c55e",
        }}
        title="Camera in use: Camera"
      >
        <svg className="w-2.5 h-2.5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
          <path d="M4 7a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v1.6l3.4-2.1a.8.8 0 0 1 1.2.7v9.6a.8.8 0 0 1-1.2.7L15 15.4V17a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7z" />
        </svg>
      </span>
      <span
        className="relative inline-flex items-center justify-center w-4 h-4 rounded-full shrink-0 animate-pulse"
        style={{
          backgroundColor: "#facc1528",
          border: "1px solid #facc15b3",
          boxShadow: "0 0 8px #facc1580",
          color: "#facc15",
        }}
        title="Microphone in use: Microsoft Teams"
      >
        <svg className="w-2.5 h-2.5 shrink-0" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 3a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zm-6 8a1 1 0 1 1 2 0 4 4 0 0 0 8 0 1 1 0 1 1 2 0 6 6 0 0 1-5 5.9V20h2a1 1 0 1 1 0 2H9a1 1 0 1 1 0-2h2v-3.1A6 6 0 0 1 6 11z" />
        </svg>
      </span>
    </span>
  );
}

const activeThemeMeta = THEMES.find((theme) => theme.id === appConfigFixture.theme)!;

/**
 * Scene 01 (s01): CompactCapsule in idle state (time only + subtle breathing glow).
 */
export function Scene01Idle() {
  return (
    <div className="relative w-full h-full flex items-center justify-center">
      <div className="island-breathing-glow" aria-hidden="true" />
      <CompactCapsule
        isExpanded={false}
        prefersReducedMotion={false}
        config={appConfigFixture}
        activeThemeMeta={activeThemeMeta}
        transientNotif={null}
        isNotifEntering={false}
        isNotifHovered={false}
        setIsNotifHovered={() => undefined}
        onViewNotification={() => undefined}
        onDismissNotification={() => undefined}
        isMediaActive={false}
        mediaSession={null}
        timeDisplay={{ timeStr: "10:42", ampm: "AM" }}
        privacyLight={null}
        hardwareMetrics={null}
        audioSpectrum={null}
        onOpenHome={() => undefined}
      />
    </div>
  );
}

/**
 * Scene 02 (s02): CompactCapsule in dual-tasking mode
 * (mediaSession playing + privacy active with mic+cam + audioSpectrum bars).
 */
export function Scene02DualTask() {
  const spectrum = useAudioSpectrum();

  return (
    <div className="relative w-full h-full flex items-center justify-center">
      <CompactCapsule
        isExpanded={false}
        prefersReducedMotion={false}
        config={appConfigFixture}
        activeThemeMeta={activeThemeMeta}
        transientNotif={null}
        isNotifEntering={false}
        isNotifHovered={false}
        setIsNotifHovered={() => undefined}
        onViewNotification={() => undefined}
        onDismissNotification={() => undefined}
        isMediaActive={true}
        mediaSession={mediaSessionFixture}
        timeDisplay={{ timeStr: "10:42", ampm: "AM" }}
        privacyLight={<DualPrivacyLight />}
        hardwareMetrics={null}
        audioSpectrum={spectrum}
        onOpenHome={() => undefined}
      />
    </div>
  );
}

/**
 * Scene 03 (s03): CompactCapsule with transient incoming call notification
 * (transientNotif={notificationFixtures[0]} and isNotifHovered={true} showing the 112px card with Answer/Decline buttons).
 */
export function Scene03IncomingCall() {
  return (
    <div className="relative w-full h-full flex items-center justify-center">
      <CompactCapsule
        isExpanded={false}
        prefersReducedMotion={false}
        config={appConfigFixture}
        activeThemeMeta={activeThemeMeta}
        transientNotif={notificationFixtures[0]}
        isNotifEntering={false}
        isNotifHovered={true}
        setIsNotifHovered={() => undefined}
        onViewNotification={() => undefined}
        onDismissNotification={() => undefined}
        isMediaActive={false}
        mediaSession={null}
        timeDisplay={{ timeStr: "10:42", ampm: "AM" }}
        privacyLight={null}
        hardwareMetrics={null}
        audioSpectrum={null}
        onOpenHome={() => undefined}
      />
    </div>
  );
}

/**
 * Scene 04 (s04): HomeSection in expanded media player mode (316x168)
 * showing mediaSessionFixture and audioSpectrumFixture.
 */
export function Scene04ExpandedMedia() {
  const spectrum = useAudioSpectrum();

  const formatSeconds = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const rem = Math.floor(secs % 60);
    return `${mins}:${String(rem).padStart(2, "0")}`;
  };

  return (
    <div className="relative w-full h-full overflow-hidden rounded-[26px]">
      <HomeSection
        config={appConfigFixture}
        activeThemeMeta={activeThemeMeta}
        showHomeNotifications={false}
        setShowHomeNotifications={() => undefined}
        setSelectedNotifId={() => undefined}
        displayNotif={null}
        notificationSnapshot={null}
        onDismissNotification={() => undefined}
        onClearAllNotifications={() => undefined}
        onViewNotification={() => undefined}
        formatRelativeTime={() => "Just now"}
        isMediaActive={true}
        mediaSession={mediaSessionFixture}
        isMediaCommandPending={false}
        onTogglePin={() => undefined}
        onMediaCommand={() => undefined}
        isDragging={false}
        dragPos={0}
        pendingSeekPos={null}
        lastServerPos={mediaSessionFixture.timeline?.position_secs ?? 92}
        lastServerTs={Date.now()}
        onSeekPointerDown={() => undefined}
        onSeekChange={() => undefined}
        onSeekPointerUp={() => undefined}
        onSeekPointerCancel={() => undefined}
        onSeekKeyUp={() => undefined}
        formatSeconds={formatSeconds}
        timeDisplay={{ timeStr: "10:42", ampm: "AM" }}
        formattedDate="Friday, October 24"
        expandedPrivacySentence={null}
        privacyState={null}
        audioSpectrum={spectrum}
        timerRemainingSecs={0}
        callDurationSecs={null}
        shelfItems={[]}
      />
    </div>
  );
}

/**
 * Scene 05 (s05): MultiTaskSection (316x168 2x2 Bento grid) with active media,
 * call duration 134 secs (02:14), focus timer 1499 secs (24:59), and reminderFixtures / shelfFixtures.
 */
export function Scene05MultiTask() {
  const formatSeconds = (secs: number) => {
    const mins = Math.floor(secs / 60);
    const rem = Math.floor(secs % 60);
    return `${mins}:${String(rem).padStart(2, "0")}`;
  };

  return (
    <div className="relative w-full h-full overflow-hidden rounded-[26px] p-2">
      <MultiTaskSection
        activeThemeMeta={activeThemeMeta}
        isMediaActive={true}
        mediaSession={mediaSessionFixture}
        onMediaCommand={() => undefined}
        onOpenMediaHome={() => undefined}
        privacyState={{
          cameraActive: false,
          cameraApp: null,
          microphoneActive: true,
          microphoneApp: "Microsoft Teams",
          isReliable: true,
        }}
        callDurationSecs={134}
        timerRemainingSecs={1499}
        isTimerPaused={false}
        timerLabel="Focus"
        onStartTimer={() => undefined}
        onPauseTimer={() => undefined}
        onResumeTimer={() => undefined}
        onStopTimer={() => undefined}
        shelfItems={shelfFixtures}
        onOpenShelf={() => undefined}
        todayNextEvent={reminderFixtures[0]}
        onOpenCalendarSection={() => undefined}
        formatSeconds={formatSeconds}
      />
    </div>
  );
}

/**
 * Scene 06 (s06): CalendarSection (316x168) populated with reminderFixtures
 * (exam in rose, deadline in rose, fest/meeting) and today selected.
 */
export function Scene06Calendar() {
  const [calendarMonth, setCalendarMonth] = useState<Date>(() => new Date());
  const [selectedCalendarDate, setSelectedCalendarDate] = useState<Date>(() => new Date());
  const [newTitle, setNewTitle] = useState("");

  return (
    <div className="relative w-full h-full overflow-hidden rounded-[26px] p-2">
      <CalendarSection
        activeThemeMeta={activeThemeMeta}
        calendarMonth={calendarMonth}
        setCalendarMonth={setCalendarMonth}
        selectedCalendarDate={selectedCalendarDate}
        setSelectedCalendarDate={setSelectedCalendarDate}
        reminders={reminderFixtures}
        newReminderTitle={newTitle}
        setNewReminderTitle={setNewTitle}
        onAddReminder={() => undefined}
        onToggleReminder={() => undefined}
        onDeleteReminder={() => undefined}
        onImportCalendar={() => undefined}
      />
    </div>
  );
}

/**
 * Scene 07 (s07): CompactCapsule with active camera + microphone privacy indicator
 * alongside a crisp sensor telemetry card (Media Foundation Sensor Monitor + WASAPI Capture -> ORBIT | Zero Hardware Lock).
 */
export function Scene07Security() {
  return (
    <div className="relative w-full h-full flex flex-col justify-between p-2.5 select-none">
      {/* Top 38px: Live CompactCapsule with Active Camera + Microphone Privacy Indicators */}
      <div className="relative w-full h-[38px] rounded-full overflow-hidden border border-white/10 bg-white/[0.03]">
        <CompactCapsule
          isExpanded={false}
          prefersReducedMotion={false}
          config={appConfigFixture}
          activeThemeMeta={activeThemeMeta}
          transientNotif={null}
          isNotifEntering={false}
          isNotifHovered={false}
          setIsNotifHovered={() => undefined}
          onViewNotification={() => undefined}
          onDismissNotification={() => undefined}
          isMediaActive={false}
          mediaSession={null}
          timeDisplay={{ timeStr: "10:42", ampm: "AM" }}
          privacyLight={<DualPrivacyLight />}
          hardwareMetrics={null}
          audioSpectrum={null}
          onOpenHome={() => undefined}
        />
      </div>

      {/* Crisp Sensor Telemetry Architecture Card */}
      <div className="rounded-xl orbit-card-3d px-2.5 py-1.5 flex flex-col justify-between bg-black/40 border border-emerald-500/25 h-[58px]">
        <div className="flex items-center justify-between text-[9px] font-bold tracking-wider uppercase text-emerald-400">
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            Sensor Telemetry Guard
          </span>
          <span className="flex items-center gap-1 text-[8.5px] font-semibold text-emerald-300 bg-emerald-500/15 px-1.5 py-0.5 rounded border border-emerald-500/30">
            🔒 Zero Hardware Lock
          </span>
        </div>
        <div className="flex items-center justify-between gap-1 text-[8.5px] font-medium text-white/80">
          <div className="flex items-center gap-1 bg-white/5 px-1.5 py-0.5 rounded border border-white/10 truncate">
            <span className="text-emerald-400 text-[10px]">●</span>
            <span className="truncate">Media Foundation</span>
          </div>
          <span className="text-white/40 text-[9px] shrink-0">→</span>
          <div className="flex items-center gap-1 bg-white/5 px-1.5 py-0.5 rounded border border-white/10 truncate">
            <span className="text-amber-400 text-[10px]">●</span>
            <span className="truncate">WASAPI Sessions</span>
          </div>
          <span className="text-white/40 text-[9px] shrink-0">→</span>
          <span className="font-bold text-cyan-300 bg-cyan-500/20 px-1.5 py-0.5 rounded border border-cyan-400/35 shrink-0">
            ORBIT
          </span>
        </div>
        <div className="text-[8px] text-white/50 flex items-center justify-between leading-none">
          <span>Passive Windows sensor audit stream</span>
          <span className="text-emerald-400 font-semibold">Camera never opened</span>
        </div>
      </div>
    </div>
  );
}

/**
 * Scene 08 (s08): HardwareSection (316x168) with hardwareMetricsFixture
 * (CPU, GPU, RAM with Boost/Trim button, Disk C:, Net Up/Down).
 */
export function Scene08Hardware() {
  const formatNetworkSpeed = (bps: number): string => {
    if (bps >= 1024 * 1024) {
      return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
    }
    return `${Math.max(1, Math.round(bps / 1024))} KB/s`;
  };

  const handleFreeMemory = async () => {
    return { freedMb: 412 };
  };

  return (
    <div className="relative w-full h-full overflow-hidden rounded-[26px] p-2">
      <HardwareSection
        activeThemeMeta={activeThemeMeta}
        hardwareMetrics={hardwareMetricsFixture}
        formatNetworkSpeed={formatNetworkSpeed}
        onFreeMemory={handleFreeMemory}
      />
    </div>
  );
}
