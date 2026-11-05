import React, { useState } from "react";
import { ThemeMeta } from "../../core/theme";
import { Music, MousePointer, Sliders, Layers, Check, ShieldCheck } from "lucide-react";

interface OnboardingOverlayProps {
  activeThemeMeta: ThemeMeta;
  firstRun: boolean;
  onComplete: (openSettings?: boolean, onlineWeatherEnabled?: boolean) => void;
}

const GUIDE_STEPS = [
  {
    title: "Welcome to ORBIT Island",
    subtitle: "Your 3D Dynamic Island for Windows",
    description: "Hover or click the compact pill to expand. It shows your clock, media, notifications, and optional widgets.",
    icon: <MousePointer className="w-4 h-4" />,
    badge: "Step 1 · Basics",
  },
  {
    title: "Navigate the island",
    subtitle: "Scroll or use the right-edge dots",
    description: "Move between Home, Calendar & Tasks, and Hardware Monitor. RAM Boost asks Windows to trim app working sets in this sign-in session.",
    icon: <Layers className="w-4 h-4" />,
    badge: "Step 2 · Navigation",
  },
  {
    title: "Media, notifications & Shelf",
    subtitle: "Local playback controls and file shortcuts",
    description: "Use media controls, open or dismiss notification previews, and drag files onto the island to add them to Shelf.",
    icon: <Music className="w-4 h-4" />,
    badge: "Step 3 · Features",
  },
  {
    title: "Settings and appearance",
    subtitle: "Gear icon or right-click menu",
    description: "Choose themes, glass style, player layout, notifications, and other preferences in ORBIT Settings.",
    icon: <Sliders className="w-4 h-4" />,
    badge: "Step 4 · Settings",
  },
];

export const OnboardingOverlay: React.FC<OnboardingOverlayProps> = ({
  activeThemeMeta,
  firstRun,
  onComplete,
}) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [accepted, setAccepted] = useState(false);
  const [onlineWeatherEnabled, setOnlineWeatherEnabled] = useState(false);
  const isPrivacyStep = firstRun && currentStep === 0;
  const guideIndex = firstRun ? currentStep - 1 : currentStep;
  const step = GUIDE_STEPS[Math.max(0, guideIndex)];
  const totalSteps = GUIDE_STEPS.length + (firstRun ? 1 : 0);
  const stepNumber = currentStep + 1;

  const handleNext = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isPrivacyStep) {
      if (accepted) setCurrentStep(1);
      return;
    }
    if (guideIndex < GUIDE_STEPS.length - 1) {
      setCurrentStep((prev) => prev + 1);
    } else {
      onComplete(true, firstRun ? onlineWeatherEnabled : undefined);
    }
  };

  const finish = (openSettings: boolean) =>
    onComplete(openSettings, firstRun ? onlineWeatherEnabled : undefined);

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="absolute inset-0 z-40 rounded-[26px] bg-slate-950/95 backdrop-blur-md flex flex-col justify-between p-3.5 select-none text-slate-100 border border-white/15 animate-fadeIn"
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center space-x-2 min-w-0">
          <div
            className="w-7 h-7 rounded-xl flex items-center justify-center orbit-card-3d shrink-0"
            style={{
              backgroundColor: `${activeThemeMeta.accent}20`,
              borderColor: `${activeThemeMeta.accent}40`,
              color: activeThemeMeta.accent,
            }}
          >
            {isPrivacyStep ? <ShieldCheck className="w-4 h-4" /> : step.icon}
          </div>
          <div className="min-w-0">
            <span className="text-[11.5px] font-bold tracking-tight text-white block leading-tight truncate">
              {isPrivacyStep ? "Terms & Privacy" : step.title}
            </span>
            <span className="text-[9px] font-semibold text-white/55 block leading-tight truncate">
              {isPrivacyStep ? "First-run setup · review before continuing" : `${step.badge} · ${step.subtitle}`}
            </span>
          </div>
        </div>
        {!isPrivacyStep && (
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); finish(true); }}
            className="text-[9.5px] font-medium text-white/50 hover:text-white px-2 py-0.5 rounded-md hover:bg-white/10 transition-colors cursor-pointer shrink-0"
            title="Skip the guide and open Settings"
          >
            Skip guide
          </button>
        )}
      </div>

      {isPrivacyStep ? (
        <div className="my-1 min-h-0 flex-1 overflow-y-auto px-1 space-y-1.5 text-[9px] leading-snug text-slate-200">
          <p><strong className="text-white">Terms of use.</strong> ORBIT is a local desktop utility. Use its system controls only when you understand their effect. RAM Boost asks Windows to trim app working sets for this session; it cannot guarantee a performance gain. ORBIT is provided as-is.</p>
          <p><strong className="text-white">Privacy.</strong> Settings, media details, notification previews, and Shelf data stay on this PC. ORBIT does not upload or sync them. Online weather is optional and contacts wttr.in only when enabled; opening GitHub links connects your browser to GitHub.</p>
          <p><strong className="text-white">Optional internet use.</strong> Live weather sends a request to wttr.in, which can see your network IP. Leave the box below clear to keep ORBIT offline.</p>
          <p><strong className="text-white">Permissions.</strong> Notification previews need Windows notification access. ORBIT reads Windows camera/microphone activity status only; it does not record either, so it does not request microphone permission.</p>
          <label className="flex items-start gap-1.5 rounded-lg border border-white/15 bg-white/5 p-1.5 cursor-pointer text-white">
            <input type="checkbox" checked={accepted} onChange={(e) => setAccepted(e.target.checked)} className="mt-0.5 accent-cyan-400" />
            <span>I agree to the ORBIT terms of use and have read this privacy notice.</span>
          </label>
          <label className="flex items-start gap-1.5 cursor-pointer text-white/75">
            <input type="checkbox" checked={onlineWeatherEnabled} onChange={(e) => setOnlineWeatherEnabled(e.target.checked)} className="mt-0.5 accent-cyan-400" />
            <span>Enable online weather (optional; wttr.in receives the request).</span>
          </label>
        </div>
      ) : (
        <div className="my-auto py-1 px-1">
          <p className="text-[10.5px] leading-relaxed text-slate-200 font-normal">{step.description}</p>
        </div>
      )}

      <div className="flex items-center justify-between pt-1 border-t border-white/10">
        <div className="flex items-center space-x-1.5">
          {Array.from({ length: totalSteps }).map((_, idx) => (
            <span
              key={idx}
              className={`h-1.5 rounded-full transition-all duration-300 ${idx === currentStep ? "w-5" : "w-1.5 bg-white/25"}`}
              style={{ backgroundColor: idx === currentStep ? activeThemeMeta.accent : undefined }}
            />
          ))}
          <span className="text-[9px] text-white/45 font-mono ml-1">{stepNumber}/{totalSteps}</span>
        </div>

        <div className="flex items-center space-x-1.5">
          {!isPrivacyStep && guideIndex > 0 && (
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setCurrentStep((prev) => prev - 1); }}
              className="px-2.5 py-1 rounded-lg text-[10px] font-semibold bg-white/10 text-white/80 hover:bg-white/20 hover:text-white transition-colors cursor-pointer"
            >Back</button>
          )}
          <button
            type="button"
            onClick={handleNext}
            disabled={isPrivacyStep && !accepted}
            className="flex items-center space-x-1 px-3 py-1 rounded-lg text-[10.5px] font-bold text-slate-950 transition-all cursor-pointer shadow-sm hover:brightness-110 active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed"
            style={{ backgroundColor: activeThemeMeta.accent }}
          >
            <span>{isPrivacyStep ? "Accept & Continue" : guideIndex === GUIDE_STEPS.length - 1 ? "Open Settings & Finish" : "Next"}</span>
            {(!isPrivacyStep && guideIndex === GUIDE_STEPS.length - 1) ? <Check className="w-3 h-3 stroke-[3]" /> : <span>→</span>}
          </button>
        </div>
      </div>
    </div>
  );
};
