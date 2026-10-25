import { useEffect, useState } from "react";
import type { AudioSpectrumState } from "@app/core/types/media";

const FFT_SIZE = 2048;
const BAND_COUNT = 20;

/**
 * Precomputes 20 log-spaced frequency band edges from 40 Hz to 16 kHz,
 * mapped to FFT bin indices bin = f * 2048 / sampleRate.
 * Exact port of make_band_edges in spectrum_dsp.rs.
 */
export function makeBandEdges(sampleRate: number): [number, number][] {
  const maxBin = FFT_SIZE / 2; // 1024
  const nyquist = sampleRate / 2;
  const upperHz = Math.max(40, Math.min(16000, nyquist));
  const lowerHz = Math.min(upperHz, 40);
  const ratio = upperHz / lowerHz;
  const edges: [number, number][] = [];
  let previous = 1;

  for (let i = 0; i < BAND_COUNT; i++) {
    const lowHz = lowerHz * Math.pow(ratio, i / BAND_COUNT);
    const highHz = lowerHz * Math.pow(ratio, (i + 1) / BAND_COUNT);
    let start = Math.floor((lowHz * FFT_SIZE) / sampleRate);
    let end = Math.ceil((highHz * FFT_SIZE) / sampleRate);
    start = Math.max(previous, Math.min(start, maxBin - 1));
    end = Math.max(start + 1, Math.min(end, maxBin + 1));
    edges.push([start, end]);
    previous = Math.min(end, maxBin);
  }
  return edges;
}

/**
 * Finds the maximum smoothed value in a given frequency range (Hz).
 * Exact port of max_in_range in spectrum_dsp.rs.
 */
export function maxInRange(
  values: number[],
  edges: [number, number][],
  sampleRate: number,
  lowHz: number,
  highHz: number,
): number {
  let max = 0;
  for (let i = 0; i < values.length; i++) {
    const [start, end] = edges[i];
    const center = ((start + end) * 0.5 * sampleRate) / FFT_SIZE;
    if (center >= lowHz && center < highHz) {
      if (values[i] > max) max = values[i];
    }
  }
  return max;
}

/**
 * Audio Spectrum DSP Analyzer engine.
 * Computes the 20 log-spaced bands with attack 0.6 / release 0.15 smoothing
 * and 4 compact bar groups from live AnalyserNode frequency data.
 */
export class SpectrumDspEngine {
  private smoothed: number[] = new Array(BAND_COUNT).fill(0);
  private bandEdges: [number, number][];
  private sampleRate: number;

  constructor(sampleRate = 48000) {
    this.sampleRate = Math.max(1, sampleRate);
    this.bandEdges = makeBandEdges(this.sampleRate);
  }

  public updateSampleRate(sampleRate: number) {
    if (sampleRate !== this.sampleRate && sampleRate > 0) {
      this.sampleRate = sampleRate;
      this.bandEdges = makeBandEdges(sampleRate);
    }
  }

  /**
   * Processes decibel frequency data from Web Audio AnalyserNode (length = 1024).
   * Normalizes -80 dB -> 0.0, -10 dB -> 1.0, applies 0.6/0.15 attack/release smoothing.
   */
  public process(floatDbArray: Float32Array): AudioSpectrumState {
    for (let i = 0; i < BAND_COUNT; i++) {
      const [start, end] = this.bandEdges[i];
      let sumMag = 0;
      let count = 0;
      for (let b = start; b < end; b++) {
        // Convert dB to linear magnitude for true spectral energy averaging
        const dbVal = floatDbArray[b] ?? -100;
        sumMag += Math.pow(10, dbVal / 20);
        count++;
      }
      const avgMag = sumMag / Math.max(1, count);
      const db = 20 * Math.log10(avgMag + 1e-9);
      const target = Math.min(1, Math.max(0, (db + 80) / 70));
      const coeff = target > this.smoothed[i] ? 0.6 : 0.15;
      this.smoothed[i] += (target - this.smoothed[i]) * coeff;
    }

    const waveBars = [...this.smoothed];
    const compactBars: [number, number, number, number] = [
      maxInRange(this.smoothed, this.bandEdges, this.sampleRate, 40, 150),
      maxInRange(this.smoothed, this.bandEdges, this.sampleRate, 150, 600),
      maxInRange(this.smoothed, this.bandEdges, this.sampleRate, 600, 2500),
      maxInRange(this.smoothed, this.bandEdges, this.sampleRate, 2500, 10000),
    ];

    const peak = Math.max(...waveBars, 0);

    return {
      peak,
      compactBars,
      waveBars,
    };
  }

  public reset() {
    this.smoothed.fill(0);
  }
}

/**
 * Singleton Audio & Spectrum Manager for ORBIT landing site.
 * Coordinates 60fps deterministic synthetic generation (default) and opt-in Web Audio synthesis.
 */
class SiteAudioSpectrumManager {
  private dspEngine = new SpectrumDspEngine(48000);
  private audioCtx: AudioContext | null = null;
  private analyser: AnalyserNode | null = null;
  private floatDbArray = new Float32Array(FFT_SIZE / 2);
  private isAudioActive = false;
  private isPlaying = false;
  private synthLoopTimer: number | null = null;
  private masterGain: GainNode | null = null;
  private audioElement: HTMLAudioElement | null = null;
  private audioSourceNode: MediaElementAudioSourceNode | null = null;
  private subscribers = new Set<(state: AudioSpectrumState) => void>();
  private playStateSubscribers = new Set<(playing: boolean) => void>();
  private rafId: number | null = null;
  private currentState: AudioSpectrumState = {
    peak: 0.65,
    compactBars: [0.72, 0.48, 0.81, 0.59],
    waveBars: new Array(20).fill(0.35),
  };

  constructor() {
    if (typeof window !== "undefined") {
      this.startLoop();
      if (typeof document !== "undefined") {
        document.addEventListener("visibilitychange", () => {
          if (document.hidden) {
            this.pause();
          }
        });
      }
    }
  }

  public getIsPlaying(): boolean {
    return this.isPlaying;
  }

  public getCurrentState(): AudioSpectrumState {
    return this.currentState;
  }

  public subscribe(cb: (state: AudioSpectrumState) => void): () => void {
    this.subscribers.add(cb);
    cb(this.currentState);
    return () => {
      this.subscribers.delete(cb);
    };
  }

  public subscribePlaying(cb: (playing: boolean) => void): () => void {
    this.playStateSubscribers.add(cb);
    cb(this.isPlaying);
    return () => {
      this.playStateSubscribers.delete(cb);
    };
  }

  private notifyPlayState() {
    for (const sub of this.playStateSubscribers) {
      sub(this.isPlaying);
    }
  }

  /**
   * Generates deterministic 60fps synthetic spectrum data matching the app's motion curves.
   */
  private generateSyntheticFrame(nowSecs: number): AudioSpectrumState {
    const t = nowSecs;
    const c0 = 0.46 + 0.38 * Math.sin(t * 5.2);
    const c1 = 0.54 + 0.36 * Math.sin(t * 6.8 + 1.2);
    const c2 = 0.66 + 0.30 * Math.sin(t * 4.4 + 2.1);
    const c3 = 0.48 + 0.38 * Math.sin(t * 7.5 + 0.8);

    const waveBars = Array.from({ length: 20 }, (_, idx) => {
      const f = 3.2 + idx * 0.35;
      const p = idx * 0.38;
      const v = 0.38 + 0.36 * Math.sin(t * f + p) + 0.16 * Math.cos(t * 1.8 + idx);
      return Math.min(1, Math.max(0.08, v));
    });

    return {
      peak: 0.65 + 0.25 * Math.sin(t * 1.5),
      compactBars: [
        Math.min(1, Math.max(0.12, c0)),
        Math.min(1, Math.max(0.12, c1)),
        Math.min(1, Math.max(0.12, c2)),
        Math.min(1, Math.max(0.12, c3)),
      ],
      waveBars,
    };
  }

  private startLoop() {
    let lastTime = performance.now();

    const loop = (now: number) => {
      // 60 fps cadence
      if (now - lastTime >= 16) {
        lastTime = now;

        if (this.isPlaying && this.analyser) {
          this.analyser.getFloatFrequencyData(this.floatDbArray);
          this.currentState = this.dspEngine.process(this.floatDbArray);
        } else {
          this.currentState = this.generateSyntheticFrame(now / 1000);
        }

        for (const sub of this.subscribers) {
          sub(this.currentState);
        }
      }

      this.rafId = requestAnimationFrame(loop);
    };

    this.rafId = requestAnimationFrame(loop);
  }

  /**
   * Initializes Web Audio Context, AnalyserNode, and master gain stage.
   */
  private initWebAudio() {
    if (this.audioCtx) return;
    const AudioContextClass =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    const ctx = new AudioContextClass();
    const analyser = ctx.createAnalyser();
    analyser.fftSize = FFT_SIZE;
    analyser.smoothingTimeConstant = 0; // Exactly per PLAN.md
    this.dspEngine.updateSampleRate(ctx.sampleRate);

    const masterGain = ctx.createGain();
    masterGain.gain.setValueAtTime(0.35, ctx.currentTime);
    masterGain.connect(analyser);
    analyser.connect(ctx.destination);

    this.audioCtx = ctx;
    this.analyser = analyser;
    this.masterGain = masterGain;

    // Optional demo-track.mp3 support
    try {
      const audio = new Audio("/audio/demo-track.mp3");
      audio.loop = true;
      audio.crossOrigin = "anonymous";
      audio.addEventListener("error", () => {
        this.audioElement = null;
      });
      this.audioElement = audio;
    } catch {
      this.audioElement = null;
    }
  }

  /**
   * Synthesizes an ambient electronic groove (kick, hi-hat, melodic synth pad)
   * routed through AnalyserNode into speakers.
   */
  private startSynthGroove() {
    if (!this.audioCtx || !this.masterGain) return;
    if (this.synthLoopTimer) return;
    const ctx = this.audioCtx;
    const masterGain = this.masterGain;

    // Chords: Dm9 -> Bbmaj7 -> Fmaj7 -> C
    const chordProgressions = [
      [293.66, 349.23, 440.0, 523.25], // D4, F4, A4, C5
      [233.08, 293.66, 349.23, 440.0], // Bb3, D4, F4, A4
      [174.61, 220.0, 261.63, 349.23],  // F3, A3, C4, F4
      [261.63, 329.63, 392.0, 523.25],  // C4, E4, G4, C5
    ];

    let step = 0;

    const playBeat = () => {
      if (!this.isPlaying || !this.audioCtx || !this.masterGain) return;
      const t = ctx.currentTime;

      // 1. Kick Drum (Bass 40-100Hz)
      const kickOsc = ctx.createOscillator();
      const kickGain = ctx.createGain();
      kickOsc.type = "sine";
      kickOsc.frequency.setValueAtTime(120, t);
      kickOsc.frequency.exponentialRampToValueAtTime(42, t + 0.12);
      kickGain.gain.setValueAtTime(0.7, t);
      kickGain.gain.exponentialRampToValueAtTime(0.001, t + 0.28);
      kickOsc.connect(kickGain);
      kickGain.connect(masterGain);
      kickOsc.start(t);
      kickOsc.stop(t + 0.3);

      // 2. Hi-Hat (Treble 8k-14kHz)
      const bufferSize = Math.floor(ctx.sampleRate * 0.05);
      const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let i = 0; i < bufferSize; i++) {
        data[i] = Math.random() * 2 - 1;
      }
      const noise = ctx.createBufferSource();
      noise.buffer = buffer;
      const filter = ctx.createBiquadFilter();
      filter.type = "highpass";
      filter.frequency.setValueAtTime(7500, t);
      const hatGain = ctx.createGain();
      hatGain.gain.setValueAtTime(0.2, t);
      hatGain.gain.exponentialRampToValueAtTime(0.001, t + 0.05);
      noise.connect(filter);
      filter.connect(hatGain);
      hatGain.connect(masterGain);
      noise.start(t);

      // 3. Synth Chord Voices (Mids 200-2500Hz)
      const chord = chordProgressions[Math.floor(step / 2) % chordProgressions.length];
      chord.forEach((freq, idx) => {
        const osc = ctx.createOscillator();
        const padFilter = ctx.createBiquadFilter();
        const padGain = ctx.createGain();

        osc.type = idx % 2 === 0 ? "sawtooth" : "triangle";
        osc.frequency.setValueAtTime(freq, t);

        padFilter.type = "lowpass";
        padFilter.frequency.setValueAtTime(1400, t);
        padFilter.frequency.exponentialRampToValueAtTime(600, t + 0.45);

        padGain.gain.setValueAtTime(0.05, t);
        padGain.gain.linearRampToValueAtTime(0.12, t + 0.08);
        padGain.gain.exponentialRampToValueAtTime(0.001, t + 0.45);

        osc.connect(padFilter);
        padFilter.connect(padGain);
        padGain.connect(masterGain);

        osc.start(t);
        osc.stop(t + 0.5);
      });

      step++;
      this.synthLoopTimer = window.setTimeout(playBeat, 460); // ~130 BPM eighth-notes
    };

    playBeat();
  }

  public async play(): Promise<void> {
    this.initWebAudio();
    if (!this.audioCtx) return;

    if (this.audioCtx.state === "suspended") {
      await this.audioCtx.resume();
    }

    this.isPlaying = true;
    this.notifyPlayState();

    // Check if mp3 source is usable
    let playedMp3 = false;
    if (this.audioElement && this.masterGain) {
      try {
        if (!this.audioSourceNode) {
          this.audioSourceNode = this.audioCtx.createMediaElementSource(this.audioElement);
          this.audioSourceNode.connect(this.masterGain);
        }
        await this.audioElement.play();
        playedMp3 = true;
      } catch {
        playedMp3 = false;
      }
    }

    if (!playedMp3) {
      this.startSynthGroove();
    }
  }

  public pause(): void {
    if (!this.isPlaying) return;
    this.isPlaying = false;
    if (this.synthLoopTimer) {
      clearTimeout(this.synthLoopTimer);
      this.synthLoopTimer = null;
    }
    if (this.audioElement) {
      this.audioElement.pause();
    }
    if (this.audioCtx && this.audioCtx.state === "running") {
      this.audioCtx.suspend().catch(() => {});
    }
    this.dspEngine.reset();
    this.notifyPlayState();
  }

  public async toggle(): Promise<boolean> {
    if (this.isPlaying) {
      this.pause();
      return false;
    }
    await this.play();
    return true;
  }
}

export const siteAudioSpectrumManager = new SiteAudioSpectrumManager();

/**
 * React hook subscribing to live AudioSpectrumState.
 * Runs 60fps synthetic by default, or live Web Audio FFT spectrum when demo audio is unmuted.
 */
export function useAudioSpectrum(active = true): AudioSpectrumState {
  const [spectrum, setSpectrum] = useState<AudioSpectrumState>(() =>
    siteAudioSpectrumManager.getCurrentState()
  );

  useEffect(() => {
    if (!active) return;
    return siteAudioSpectrumManager.subscribe(setSpectrum);
  }, [active]);

  return spectrum;
}

/**
 * React hook reporting whether demo audio is playing.
 */
export function useAudioPlaying(): boolean {
  const [isPlaying, setIsPlaying] = useState<boolean>(() =>
    siteAudioSpectrumManager.getIsPlaying()
  );

  useEffect(() => {
    return siteAudioSpectrumManager.subscribePlaying(setIsPlaying);
  }, []);

  return isPlaying;
}

export function playDemoAudio(): Promise<void> {
  return siteAudioSpectrumManager.play();
}

export function pauseDemoAudio(): void {
  siteAudioSpectrumManager.pause();
}

export function toggleDemoAudio(): Promise<boolean> {
  return siteAudioSpectrumManager.toggle();
}
