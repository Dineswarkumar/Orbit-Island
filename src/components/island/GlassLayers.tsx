import React, { useId, useMemo } from "react";

export type GlassMode = "glass" | "frosted" | "smoked";
export type GlassTone = "dark" | "light";

/** Size (in CSS px) of the native window the live desktop capture was taken from. */
export const CAPTURE_WINDOW_WIDTH = 860;
export const CAPTURE_WINDOW_HEIGHT = 188;

// The SVG filter region is deliberately larger than any island state so the shell can
// animate between sizes without the refraction layer ever clipping it.
const REGION_PAD = 40;
const REGION_W = CAPTURE_WINDOW_WIDTH + REGION_PAD * 2;
const REGION_H = CAPTURE_WINDOW_HEIGHT + REGION_PAD * 2;

// Width of the rim band (px) where the lens bends the backdrop.
const LENS_EDGE = 15;
const LENS_STEPS = [0, 0.18, 0.38, 0.62, 1];

interface LensProfile {
  /** Peak refraction at the rim, in px. */
  scale: number;
  blur: number;
  saturate: number;
  brightness: number;
}

const LENS: Record<GlassMode, LensProfile> = {
  // Normal: mostly clear with a restrained optical edge.
  glass: { scale: 6, blur: 0.5, saturate: 1.06, brightness: 1.01 },
  // Blurred: a soft privacy frost without obscuring the desktop.
  frosted: { scale: 2, blur: 3, saturate: 1.04, brightness: 1.02 },
  // Realistic: a subtle natural tint with moderate background softness.
  smoked: { scale: 4, blur: 1, saturate: 1.1, brightness: 1.01 },
};

const profile = (t: number) => Math.pow(1 - t, 2.2);

/**
 * Builds one channel of the displacement map as an SVG gradient. Neutral (128) in the
 * middle of the shell, ramping towards the rim so only the edge of the glass bends light,
 * exactly like the bevel of a thick lens.
 */
const buildChannelStops = (
  regionLen: number,
  shellLen: number,
  channel: "r" | "g",
): string => {
  const a = REGION_PAD; // shell start inside the region
  const b = REGION_PAD + shellLen; // shell end
  const edge = Math.min(LENS_EDGE, shellLen / 2 - 1);
  const D = 127;
  const color = (v: number) => {
    const c = Math.max(0, Math.min(255, Math.round(v)));
    return channel === "r" ? `rgb(${c},0,0)` : `rgb(0,${c},0)`;
  };
  const stops: Array<[number, number]> = [[0, 128 - D]];
  for (const t of LENS_STEPS) {
    stops.push([a + edge * t, 128 - D * profile(t)]);
  }
  for (const t of [...LENS_STEPS].reverse()) {
    stops.push([b - edge * t, 128 + D * profile(t)]);
  }
  stops.push([regionLen, 128 + D]);
  return stops
    .map(
      ([pos, v]) =>
        `<stop offset="${Math.max(0, Math.min(1, pos / regionLen)).toFixed(4)}" stop-color="${color(v)}"/>`,
    )
    .join("");
};

const buildLensMap = (shellLen: number, axis: "x" | "y"): string => {
  const regionLen = axis === "x" ? REGION_W : REGION_H;
  const stops = buildChannelStops(regionLen, shellLen, axis === "x" ? "r" : "g");
  const [x2, y2] = axis === "x" ? ["1", "0"] : ["0", "1"];
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${REGION_W}" height="${REGION_H}" viewBox="0 0 ${REGION_W} ${REGION_H}">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="${x2}" y2="${y2}">${stops}</linearGradient></defs>` +
    `<rect width="${REGION_W}" height="${REGION_H}" fill="url(#g)"/></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
};

interface GlassLayersProps {
  mode: GlassMode;
  tone: GlassTone;
  /** Current rendered size of the island shell, in px. */
  width: number;
  height: number;
  /** Live capture of the desktop behind the island window (null until first sample). */
  backdropUrl: string | null;
  /** Position of the shell inside the 860x188 native window the capture was taken of. */
  offsetX: number;
  offsetY: number;
  /**
   * When the host surface is an ordinary opaque window (Settings preview), the browser can
   * blur what is behind it natively - no capture needed.
   */
  nativeBackdrop?: boolean;
}

/**
 * The full liquid-glass stack for the island shell, bottom to top:
 *  1. backdrop  - live, lens-refracted, blurred capture of the desktop behind the island
 *  2. scrim     - density + tint, tone aware so ink always has contrast
 *  3. sheen     - top light falloff and lower caustic for volume
 *  4. rim       - thin directional specular edge
 */
export const GlassLayers: React.FC<GlassLayersProps> = React.memo(({
  mode,
  tone,
  width,
  height,
  backdropUrl,
  offsetX,
  offsetY,
  nativeBackdrop = false,
}) => {
  const rawId = useId().replace(/[^a-zA-Z0-9_-]/g, "_");
  const filterId = `orbit-lens-${rawId}`;
  const lens = LENS[mode];

  const mapX = useMemo(() => buildLensMap(Math.round(width), "x"), [width]);
  const mapY = useMemo(() => buildLensMap(Math.round(height), "y"), [height]);

  const hasLiveBackdrop = !nativeBackdrop && !!backdropUrl;

  return (
    <>
      {hasLiveBackdrop && (
        <>
          <svg
            aria-hidden="true"
            focusable="false"
            style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}
          >
            <filter
              id={filterId}
              filterUnits="userSpaceOnUse"
              x={-REGION_PAD}
              y={-REGION_PAD}
              width={REGION_W}
              height={REGION_H}
              colorInterpolationFilters="sRGB"
            >
              <feImage
                href={mapX}
                x={-REGION_PAD}
                y={-REGION_PAD}
                width={REGION_W}
                height={REGION_H}
                preserveAspectRatio="none"
                result="lensX"
              />
              <feImage
                href={mapY}
                x={-REGION_PAD}
                y={-REGION_PAD}
                width={REGION_W}
                height={REGION_H}
                preserveAspectRatio="none"
                result="lensY"
              />
              <feComposite
                in="lensX"
                in2="lensY"
                operator="arithmetic"
                k1={0}
                k2={1}
                k3={1}
                k4={0}
                result="lensMap"
              />
              <feDisplacementMap
                in="SourceGraphic"
                in2="lensMap"
                scale={lens.scale}
                xChannelSelector="R"
                yChannelSelector="G"
              />
            </filter>
          </svg>

          <div
            className="orbit-glass-backdrop"
            aria-hidden="true"
            style={{
              filter: `url(#${filterId}) blur(${lens.blur}px) saturate(${lens.saturate}) brightness(${lens.brightness})`,
            }}
          >
            {/* Oversized under-layer so blur/refraction near the window edge never samples transparency */}
            <img
              src={backdropUrl as string}
              alt=""
              draggable={false}
              style={{
                position: "absolute",
                left: -offsetX - 70,
                top: -offsetY - 50,
                width: CAPTURE_WINDOW_WIDTH + 140,
                height: CAPTURE_WINDOW_HEIGHT + 100,
                maxWidth: "none",
              }}
            />
            {/* 1:1 pixel-aligned capture */}
            <img
              src={backdropUrl as string}
              alt=""
              draggable={false}
              style={{
                position: "absolute",
                left: -offsetX,
                top: -offsetY,
                width: CAPTURE_WINDOW_WIDTH,
                height: CAPTURE_WINDOW_HEIGHT,
                maxWidth: "none",
              }}
            />
          </div>
        </>
      )}

      {nativeBackdrop && (
        <div
          className="orbit-glass-native"
          aria-hidden="true"
          style={{
            backdropFilter: `blur(${Math.max(1, lens.blur * 1.6)}px) saturate(${lens.saturate})`,
            WebkitBackdropFilter: `blur(${Math.max(1, lens.blur * 1.6)}px) saturate(${lens.saturate})`,
          }}
        />
      )}

      <div className="orbit-glass-scrim" aria-hidden="true" data-tone={tone} />
      <div className="orbit-glass-sheen" aria-hidden="true" />
      <div className="orbit-glass-rim" aria-hidden="true" />
    </>
  );
});

GlassLayers.displayName = "GlassLayers";

/**
 * Picks light/dark ink from the measured luminance of the desktop behind the island.
 * Hysteresis keeps the ink from flickering when the backdrop hovers around mid-grey.
 */
export const nextGlassTone = (previous: GlassTone, luminance: number): GlassTone => {
  if (previous === "dark") {
    return luminance > 0.6 ? "light" : "dark";
  }
  return luminance < 0.47 ? "dark" : "light";
};
