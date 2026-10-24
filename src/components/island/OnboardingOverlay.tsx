import React, { useState } from "react";
import { ThemeMeta } from "../../core/theme";
import { Music, MousePointer, Sliders, Layers, Check } from "lucide-react";

interface OnboardingOverlayProps {
  activeThemeMeta: ThemeMeta;
  onComplete: (openSettings?: boolean) => void;
}

export const OnboardingOverlay: React.FC<OnboardingOverlayProps> = ({
  activeThemeMeta,
  onComplete,
}) => {
  const [currentStep, setCurrentStep] = useState<number>(0);

  const steps = [
    {
      title: "Welcome to ORBIT Island",
      subtitle: "Your 3D Dynamic Island for Windows",
      description:
        "Hover or click the compact pill at the top of your screen to expand. In compact mode, see your live clock, weather or custom badge, and glowing camera/mic privacy lights.",
      icon: <MousePointer className="w-4 h-4" style={{ color: activeThemeMeta.accent }} />,
      badge: "Step 1 · Basics",
    },
    {
      title: "3D Drum Carousel",
      subtitle: "Scroll Wheel or Right-Edge Dots",
      description:
        "While expanded, scroll your mouse wheel up/down (or click the 3 dots on the right edge) to flip between Home, Calendar & Tasks, and Hardware Monitor with 1-click RAM Boost.",
      icon: <Layers className="w-4 h-4" style={{ color: activeThemeMeta.accent }} />,
      badge: "Step 2 · Navigation",
    },
    {
      title: "Live Audio, Notifications & Shelf",
      subtitle: "Frequency Waves & Instant App Jump",
      description:
        "Music waves react to real audio frequency. Click any notification or Open to jump straight to that app or browser tab, or drag files onto the island to stash them in Shelf.",
      icon: <Music className="w-4 h-4" style={{ color: activeThemeMeta.accent }} />,
      badge: "Step 3 · Smart Actions",
    },
    {
      title: "Opening ORBIT Settings",
      subtitle: "Gear Icon (⚙) or Right-Click Menu",
      description:
        "Click the ⚙ Settings icon in the top-right corner of the expanded island, OR Right-Click anywhere on the island to access themes, timers, fonts, and full ORBIT Settings!",
      icon: <Sliders className="w-4 h-4" style={{ color: activeThemeMeta.accent }} />,
      badge: "Step 4 · Settings & Menu",
    },
  ];

  const step = steps[currentStep];

  const handleNext = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (currentStep < steps.length - 1) {
      setCurrentStep((prev) => prev + 1);
    } else {
      onComplete(true);
    }
  };

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="absolute inset-0 z-40 rounded-[26px] bg-slate-950/95 backdrop-blur-md flex flex-col justify-between p-3.5 select-none text-slate-100 border border-white/15 animate-fadeIn"
    >
      {/* Step Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2">
          <div
            className="w-7 h-7 rounded-xl flex items-center justify-center orbit-card-3d shrink-0"
            style={{
              backgroundColor: `${activeThemeMeta.accent}20`,
              borderColor: `${activeThemeMeta.accent}40`,
            }}
          >
            {step.icon}
          </div>
          <div className="min-w-0">
            <span className="text-[11.5px] font-bold tracking-tight text-white block leading-tight truncate">
              {step.title}
            </span>
            <span className="text-[9px] font-semibold text-white/55 block leading-tight truncate">
              {step.badge} · {step.subtitle}
            </span>
          </div>
        </div>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onComplete(true);
          }}
          className="text-[9.5px] font-medium text-white/50 hover:text-white px-2 py-0.5 rounded-md hover:bg-white/10 transition-colors cursor-pointer shrink-0"
          title="Skip tutorial and open Settings"
        >
          Skip & Open Settings
        </button>
      </div>

      {/* Step Body */}
      <div className="my-auto py-1 px-1">
        <p className="text-[10.5px] leading-relaxed text-slate-200 font-normal">
          {step.description}
        </p>
      </div>

      {/* Step Footer Navigation */}
      <div className="flex items-center justify-between pt-1 border-t border-white/10">
        {/* Step dots */}
        <div className="flex items-center space-x-1.5">
          {steps.map((_, idx) => (
            <button
              key={idx}
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setCurrentStep(idx);
              }}
              className={`h-1.5 rounded-full transition-all duration-300 cursor-pointer ${
                idx === currentStep ? "w-5" : "w-1.5 bg-white/25 hover:bg-white/45"
              }`}
              style={{
                backgroundColor: idx === currentStep ? activeThemeMeta.accent : undefined,
              }}
              title={`Go to Step ${idx + 1}`}
            />
          ))}
          <span className="text-[9px] text-white/45 font-mono ml-1">
            {currentStep + 1}/{steps.length}
          </span>
        </div>

        <div className="flex items-center space-x-1.5">
          {currentStep > 0 && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setCurrentStep((prev) => prev - 1);
              }}
              className="px-2.5 py-1 rounded-lg text-[10px] font-semibold bg-white/10 text-white/80 hover:bg-white/20 hover:text-white transition-colors cursor-pointer"
            >
              Back
            </button>
          )}
          <button
            type="button"
            onClick={handleNext}
            className="flex items-center space-x-1 px-3 py-1 rounded-lg text-[10.5px] font-bold text-slate-950 transition-all cursor-pointer shadow-sm hover:brightness-110 active:scale-95"
            style={{ backgroundColor: activeThemeMeta.accent }}
          >
            <span>
              {currentStep === steps.length - 1 ? "Open Settings & Finish" : "Next"}
            </span>
            {currentStep === steps.length - 1 ? (
              <Check className="w-3 h-3 stroke-[3]" />
            ) : (
              <span>→</span>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
