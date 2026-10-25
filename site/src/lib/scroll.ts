import gsap from "gsap";
import { CustomEase } from "gsap/CustomEase";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Lenis from "lenis";

let pluginsRegistered = false;

export function registerScrollPlugins() {
  if (pluginsRegistered) return;

  gsap.registerPlugin(ScrollTrigger, CustomEase);
  CustomEase.create("orbitSpring", "M0,0 C0.22,1.26 0.36,1 1,1");
  CustomEase.create("orbitSettle", "M0,0 C0.22,1.15 0.36,1 1,1");
  pluginsRegistered = true;
}

export function initSmoothScroll() {
  registerScrollPlugins();
  gsap.ticker.lagSmoothing(0);

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  let lenis: Lenis | null = null;

  const tick = (time: number) => lenis?.raf(time * 1000);
  const stopLenis = () => {
    if (!lenis) return;
    gsap.ticker.remove(tick);
    lenis.destroy();
    lenis = null;
  };
  const startLenis = () => {
    if (reducedMotion.matches || lenis) return;
    lenis = new Lenis();
    lenis.on("scroll", ScrollTrigger.update);
    gsap.ticker.add(tick);
  };
  const syncMotionPreference = () => {
    if (reducedMotion.matches) stopLenis();
    else startLenis();
    ScrollTrigger.refresh();
  };

  startLenis();
  reducedMotion.addEventListener("change", syncMotionPreference);

  return () => {
    reducedMotion.removeEventListener("change", syncMotionPreference);
    stopLenis();
  };
}
