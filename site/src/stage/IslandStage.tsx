import { useLayoutEffect, useRef, type ReactNode } from "react";
import gsap from "gsap";
import { sections } from "../content/sections";
import { initSmoothScroll, registerScrollPlugins } from "../lib/scroll";
import {
  useAudioPlaying,
  toggleDemoAudio,
  pauseDemoAudio,
} from "../audio/analyser";
import {
  Scene01Idle,
  Scene02DualTask,
  Scene03IncomingCall,
  Scene04ExpandedMedia,
  Scene05MultiTask,
  Scene06Calendar,
  Scene07Security,
  Scene08Hardware,
} from "./IslandScenes";

type IslandStageProps = {
  children?: ReactNode;
};

const shellStates = sections.map(({ width, height, radius }) => ({
  width,
  height,
  borderRadius: radius,
}));

export function IslandStage({ children }: IslandStageProps) {
  const scrollerRef = useRef<HTMLElement>(null);
  const shellRef = useRef<HTMLDivElement>(null);
  const isPlayingAudio = useAudioPlaying();

  useLayoutEffect(() => {
    registerScrollPlugins();
    const stopSmoothScroll = initSmoothScroll();
    const scroller = scrollerRef.current;
    const shell = shellRef.current;
    if (!scroller || !shell) return stopSmoothScroll;
    let removeScaleListener: () => void = () => {};

    const context = gsap.context(() => {
      const sceneLayers = gsap.utils.toArray<HTMLElement>("[data-scene-layer]");
      const copyPanels = gsap.utils.toArray<HTMLElement>("[data-copy-panel]");
      const stageAudioControl = scroller.querySelector<HTMLElement>("[data-audio-stage-control]");
      const scaleShell = () => {
        const preferred = window.innerWidth / 560;
        shell.style.setProperty("--island-scale", String(Math.min(2.6, Math.max(1.6, preferred))));
      };
      scaleShell();
      window.addEventListener("resize", scaleShell);
      removeScaleListener = () => window.removeEventListener("resize", scaleShell);

      gsap.set(sceneLayers, { autoAlpha: 0 });
      gsap.set(copyPanels, { autoAlpha: 0 });
      gsap.set([sceneLayers[0], copyPanels[0]], { autoAlpha: 1 });
      gsap.set(shell, shellStates[0]);
      if (stageAudioControl) {
        gsap.set(stageAudioControl, { autoAlpha: 0, y: 12, pointerEvents: "none" });
      }

      const timeline = gsap.timeline({
        defaults: { ease: "orbitSettle" },
        scrollTrigger: {
          trigger: scroller,
          start: "top top",
          end: "bottom bottom",
          scrub: 0.6,
          snap: { snapTo: "labels", duration: 0.5, ease: "power2.inOut" },
          onLeave: () => {
            pauseDemoAudio();
          },
          onLeaveBack: () => {
            pauseDemoAudio();
          },
          onUpdate: (self) => {
            if (self.progress < 0.1 || self.progress > 0.6) {
              pauseDemoAudio();
            }
          },
        },
      });

      sections.forEach((section, index) => {
        const label = `s${section.id}`;
        timeline.addLabel(label, index);
        if (index === sections.length - 1) return;

        const nextLabel = `s${sections[index + 1].id}`;
        timeline.to(shell, { ...shellStates[index + 1], duration: 1 }, label);
        timeline.to(sceneLayers[index], { autoAlpha: 0, duration: 0.4 }, `${label}+=0.4`);

        if (index + 1 === 2) {
          // Scene 03 (s03): 3D rotateX(12deg) -> 0deg perspective tween on entry
          timeline.fromTo(
            sceneLayers[index + 1],
            { autoAlpha: 0, rotateX: 12, transformPerspective: 900 },
            { autoAlpha: 1, rotateX: 0, duration: 0.45, ease: "orbitSpring" },
            `${label}+=0.55`,
          );
        } else {
          timeline.fromTo(
            sceneLayers[index + 1],
            { autoAlpha: 0, rotateX: 0 },
            { autoAlpha: 1, duration: 0.4 },
            `${label}+=0.6`,
          );
        }

        if (index + 1 === 3) {
          // Scene 04 (s04): Cover art parallax tween on entry
          const coverArt = sceneLayers[index + 1].querySelector(".orbit-card-3d");
          if (coverArt) {
            timeline.fromTo(
              coverArt,
              { y: 8, scale: 0.94 },
              { y: 0, scale: 1, duration: 0.45, ease: "power2.out" },
              `${label}+=0.55`,
            );
          }
        }

        if (stageAudioControl) {
          if (index + 1 === 3) {
            // Entering Scene 04
            timeline.to(
              stageAudioControl,
              { autoAlpha: 1, y: 0, pointerEvents: "auto", duration: 0.35, ease: "orbitSpring" },
              `${label}+=0.6`,
            );
          } else if (index === 3) {
            // Leaving Scene 04
            timeline.to(
              stageAudioControl,
              { autoAlpha: 0, y: 12, pointerEvents: "none", duration: 0.3, ease: "power2.in" },
              `${label}+=0.3`,
            );
          }
        }

        if (index + 1 === 4) {
          // Scene 05 (s05): 2x2 Bento grid cards stagger in
          const bentoCards = sceneLayers[index + 1].querySelectorAll(".orbit-card-3d");
          if (bentoCards.length > 0) {
            timeline.fromTo(
              bentoCards,
              { y: 12, autoAlpha: 0, scale: 0.96 },
              { y: 0, autoAlpha: 1, scale: 1, duration: 0.35, stagger: 0.06, ease: "orbitSpring" },
              `${label}+=0.55`,
            );
          }
        }

        if (index + 1 === 5) {
          // Scene 06 (s06): Calendar date hero and month matrix pop in
          const calCards = sceneLayers[index + 1].querySelectorAll(".orbit-card-3d");
          if (calCards.length > 0) {
            timeline.fromTo(
              calCards,
              { y: 10, autoAlpha: 0 },
              { y: 0, autoAlpha: 1, duration: 0.38, stagger: 0.05, ease: "orbitSpring" },
              `${label}+=0.55`,
            );
          }
        }

        if (index + 1 === 7) {
          // Scene 08 (s08): 2x3 Hardware telemetry cards stagger in
          const hwCards = sceneLayers[index + 1].querySelectorAll(".orbit-card-3d");
          if (hwCards.length > 0) {
            timeline.fromTo(
              hwCards,
              { y: 10, autoAlpha: 0 },
              { y: 0, autoAlpha: 1, duration: 0.32, stagger: 0.04, ease: "power2.out" },
              `${label}+=0.55`,
            );
          }
        }

        timeline.to(copyPanels[index], { autoAlpha: 0, duration: 0.3 }, `${label}+=0.35`);
        timeline.set(copyPanels[index + 1], { autoAlpha: 1 }, `${label}+=0.6`);
        timeline.fromTo(
          copyPanels[index + 1].querySelector("[data-copy-tag]"),
          { autoAlpha: 0, x: -18 },
          { autoAlpha: 1, x: 0, duration: 0.3, ease: "orbitSpring" },
          `${label}+=0.6`,
        );
        timeline.fromTo(
          copyPanels[index + 1].querySelector("[data-copy-headline]"),
          { yPercent: 115 },
          { yPercent: 0, duration: 0.48, ease: "orbitSpring" },
          `${label}+=0.6`,
        );
        timeline.fromTo(
          copyPanels[index + 1].querySelector("[data-copy-description]"),
          { autoAlpha: 0, y: 10 },
          { autoAlpha: 1, y: 0, duration: 0.35, ease: "power1.out" },
          `${label}+=0.74`,
        );

        // Keep the next label at the scene boundary even when its content tweens overlap.
        timeline.addLabel(nextLabel, index + 1);
      });

      return () => window.removeEventListener("resize", scaleShell);
    }, scroller);

    return () => {
      pauseDemoAudio();
      removeScaleListener();
      context.revert();
      stopSmoothScroll();
    };
  }, []);

  return (
    <main className="island-scroll-container" ref={scrollerRef}>
      <section className="island-stage" aria-label="ORBIT features">
        <div className="island-stage-inner">
          <div className="island-copy-stack" aria-live="polite">
            {sections.map((section, index) => (
              <article className="island-copy-panel" data-copy-panel key={section.id}>
                <p className="island-copy-tag" data-copy-tag>
                  <span>{section.tag}</span>
                </p>
                <div className="island-copy-headline-mask">
                  <h1 className="island-copy-headline" data-copy-headline>{section.headline}</h1>
                </div>
                <p className="island-copy-description" data-copy-description>{section.description}</p>
                {section.id === "04" && (
                  <div className="island-audio-action">
                    <button
                      type="button"
                      onClick={() => toggleDemoAudio()}
                      className={`island-audio-pill ${isPlayingAudio ? "is-playing" : ""}`}
                      aria-label={isPlayingAudio ? "Mute Demo Audio" : "Play Demo Audio"}
                    >
                      <span className={`island-audio-pill-dot ${isPlayingAudio ? "animate-pulse" : ""}`} />
                      <span>{isPlayingAudio ? "⏸ Mute Demo Audio" : "▶ Play Demo Audio"}</span>
                      <span className="island-audio-pill-badge">
                        {isPlayingAudio ? "REAL FFT (WEB AUDIO)" : "PREVIEW"}
                      </span>
                    </button>
                  </div>
                )}
                <span className="island-copy-index">{section.id} / 08</span>
                {index === 0 && <span className="sr-only">Scroll to explore ORBIT features.</span>}
              </article>
            ))}
          </div>

          <div className="island-stage-visual">
            <div className="island-shell-frame" ref={shellRef}>
              {sections.map((section, index) => (
                <div
                  aria-hidden="true"
                  className="island-scene-layer theme-obsidian transparency-glass orbit-font-inter"
                  data-scene-layer
                  key={section.id}
                >
                  {index === 0 ? (
                    children ?? <Scene01Idle />
                  ) : index === 1 ? (
                    <Scene02DualTask />
                  ) : index === 2 ? (
                    <Scene03IncomingCall />
                  ) : index === 3 ? (
                    <Scene04ExpandedMedia />
                  ) : index === 4 ? (
                    <Scene05MultiTask />
                  ) : index === 5 ? (
                    <Scene06Calendar />
                  ) : index === 6 ? (
                    <Scene07Security />
                  ) : index === 7 ? (
                    <Scene08Hardware />
                  ) : null}
                </div>
              ))}
            </div>
            <div className="island-stage-audio-control" data-audio-stage-control>
              <button
                type="button"
                onClick={() => toggleDemoAudio()}
                className={`island-audio-pill ${isPlayingAudio ? "is-playing" : ""}`}
                aria-label={isPlayingAudio ? "Mute Demo Audio" : "Play Demo Audio"}
              >
                <span className={`island-audio-pill-dot ${isPlayingAudio ? "animate-pulse" : ""}`} />
                <span>{isPlayingAudio ? "⏸ Mute Demo Audio" : "▶ Play Demo Audio"}</span>
                <span className="island-audio-pill-badge">
                  {isPlayingAudio ? "REAL FFT" : "PREVIEW"}
                </span>
              </button>
            </div>
            <p className="island-stage-hint">SCROLL TO EXPLORE <span aria-hidden="true">↓</span></p>
          </div>
        </div>
      </section>
    </main>
  );
}
