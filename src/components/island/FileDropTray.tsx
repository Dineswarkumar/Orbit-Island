import React, { RefObject } from "react";
import {
  AppWindow,
  Archive,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  FolderOpen,
  Printer,
  X,
  Globe2,
} from "lucide-react";
import { PinnedDestination } from "../../core/types/config";

export type FileDropAction = "shelf" | "open" | "open_with" | "reveal" | "print" | `website:${string}`;

const targets: Array<{
  id: FileDropAction;
  title: string;
  detail: string;
  Icon: typeof Archive;
}> = [
  { id: "shelf", title: "Save to Shelf", detail: "Keep a reference or copy", Icon: Archive },
  { id: "open", title: "Open", detail: "Use the default app", Icon: AppWindow },
  { id: "open_with", title: "Open with…", detail: "Choose an installed app", Icon: AppWindow },
  { id: "reveal", title: "File Explorer", detail: "Show the file location", Icon: FolderOpen },
  { id: "print", title: "Print", detail: "Send to the Windows printer", Icon: Printer },
];

interface FileDropTrayProps {
  fileCount: number;
  storageMode: "reference" | "copy";
  pinnedDestinations: PinnedDestination[];
  selected: FileDropAction;
  message: string | null;
  error: boolean;
  scrollRef: RefObject<HTMLDivElement>;
  onSelect: (action: FileDropAction) => void;
  onClose: () => void;
}

export const FileDropTray: React.FC<FileDropTrayProps> = ({
  fileCount,
  storageMode,
  pinnedDestinations,
  selected,
  message,
  error,
  scrollRef,
  onSelect,
  onClose,
}) => (
  <div className="absolute inset-0 z-[30] flex flex-col px-5 py-2.5 select-none">
    <div className="flex items-center justify-between gap-3 shrink-0">
      <div className="min-w-0">
        <div className="text-[13px] font-bold tracking-tight text-[var(--island-text)]">
          Drop to send
        </div>
        <div className="text-[10px] text-[var(--island-muted)]">
          {message ?? `${fileCount} ${fileCount === 1 ? "file" : "files"} · ${storageMode === "copy" ? "Managed copy" : "Reference only"}`}
        </div>
      </div>
      <button
        type="button"
        onClick={onClose}
        className="w-7 h-7 rounded-full flex items-center justify-center border border-white/15 bg-black/10 text-white/75 hover:text-white hover:bg-white/10 transition-colors"
        aria-label="Close file drop tray"
      >
        <X className="w-3.5 h-3.5" />
      </button>
    </div>

    <div className="flex items-center gap-1.5 mt-2 min-h-0 flex-1">
      <button
        type="button"
        onClick={() => scrollRef.current?.scrollBy({ left: -190, behavior: "smooth" })}
        className="w-6 h-9 shrink-0 rounded-lg border border-white/10 bg-black/10 text-white/65 hover:text-white hover:bg-white/10"
        aria-label="Scroll drop targets left"
      >
        <ChevronLeft className="w-4 h-4 mx-auto" />
      </button>
      <div
        ref={scrollRef}
        data-orbit-drop-scroll="true"
        className="flex-1 flex items-stretch gap-2 overflow-x-auto overscroll-x-contain scroll-smooth py-0.5"
        style={{ scrollbarWidth: "none" }}
      >
        {[...targets, ...pinnedDestinations.map((destination) => ({ id: `website:${destination.id}` as FileDropAction, title: destination.name, detail: "Open · attach from Shelf", Icon: Globe2 }))].map(({ id, title, detail, Icon }) => (
          <button
            type="button"
            key={id}
            data-orbit-drop-action={id}
            onClick={() => onSelect(id)}
            className={`shrink-0 min-w-[126px] max-w-[148px] flex-1 rounded-xl border px-2.5 py-2 text-left transition-[background-color,border-color,transform] duration-100 ${
              selected === id
                ? "border-[var(--theme-accent)] bg-[color-mix(in_srgb,var(--theme-accent)_24%,transparent)] shadow-[0_0_16px_color-mix(in_srgb,var(--theme-accent)_22%,transparent)]"
                : "border-white/15 bg-black/10 hover:bg-white/10"
            }`}
          >
            <div className="flex items-center gap-2">
              <span className="w-7 h-7 rounded-lg bg-white/10 flex items-center justify-center shrink-0">
                <Icon className="w-4 h-4 text-[var(--theme-accent)]" />
              </span>
              <span className="min-w-0">
                <span className="block text-[10px] font-semibold text-[var(--island-text)] truncate">{title}</span>
                <span className="block text-[8px] leading-tight text-[var(--island-muted)] truncate">{detail}</span>
              </span>
            </div>
          </button>
        ))}
      </div>
      <button
        type="button"
        onClick={() => scrollRef.current?.scrollBy({ left: 190, behavior: "smooth" })}
        className="w-6 h-9 shrink-0 rounded-lg border border-white/10 bg-black/10 text-white/65 hover:text-white hover:bg-white/10"
        aria-label="Scroll drop targets right"
      >
        <ChevronRight className="w-4 h-4 mx-auto" />
      </button>
    </div>
    <div className={`h-3.5 mt-0.5 text-[9px] flex items-center gap-1 ${error ? "text-rose-200" : "text-emerald-200"}`}>
      {message && !error && <CheckCircle2 className="w-3 h-3" />}
      <span className="truncate">{message && error ? message : message && "Drop finished"}</span>
    </div>
  </div>
);
