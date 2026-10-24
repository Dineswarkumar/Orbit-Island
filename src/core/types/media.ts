export interface MediaTrack {
  title: string;
  artist: string;
  album: string;
  thumbnail_base64: string | null;
}

export interface MediaTimeline {
  position_secs: number;
  duration_secs: number;
}

export interface MediaControls {
  can_play: boolean;
  can_pause: boolean;
  can_skip_next: boolean;
  can_skip_previous: boolean;
  can_seek: boolean;
}

export interface MediaSourceInfo {
  source_id: string;
  source_title: string;
  is_playing: boolean;
  is_pinned: boolean;
}

export interface MediaSessionSnapshot {
  source_id: string;
  source_title: string;
  playback_status: "playing" | "paused" | "stopped" | string;
  track: MediaTrack;
  timeline: MediaTimeline | null;
  controls: MediaControls;
  is_pinned: boolean;
}

export interface AudioSpectrumState {
  peak: number;
  compactBars: [number, number, number, number];
  waveBars: number[];
}

