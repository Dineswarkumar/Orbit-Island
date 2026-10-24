export interface ReminderItem {
  id: string;
  title: string;
  dueTimestampMs: number;
  isCompleted: boolean;
  source: "local" | "google" | "outlook" | "academic" | "office" | "ics" | string;
  category:
    | "task"
    | "event"
    | "reminder"
    | "deadline"
    | "exam"
    | "holiday"
    | "meeting"
    | "class"
    | string;
  createdAt: number;
}
