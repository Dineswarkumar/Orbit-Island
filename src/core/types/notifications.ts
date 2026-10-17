export interface NotificationAction {
  id: string;
  label: string;
  isSupported: boolean;
}

export interface NotificationItem {
  id: number;
  appName: string;
  appId: string;
  title: string;
  body: string;
  timestampMs: number;
  isCritical: boolean;
  count: number;
  actions: NotificationAction[];
  appIcon?: string | null;
  underlyingIds?: number[];
}

export interface NotificationSnapshot {
  accessStatus: "allowed" | "denied" | "unspecified" | "unsupported";
  activeNotification: NotificationItem | null;
  notifications: NotificationItem[];
  overflowCount: number;
  totalCount: number;
}

export interface PrivacyState {
  cameraActive: boolean;
  cameraApp: string | null;
  microphoneActive: boolean;
  microphoneApp: string | null;
  isReliable: boolean;
}
