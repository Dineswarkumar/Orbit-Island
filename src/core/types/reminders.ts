export interface ReminderItem {
  id: string;
  title: string;
  dueTimestampMs: number;
  isCompleted: boolean;
  source: string; // "local", "google", "outlook"
  category: string; // "task", "event", "reminder", "deadline"
  createdAt: number;
}
