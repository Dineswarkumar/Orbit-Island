import React, { useState } from "react";
import { ShelfItem } from "../../core/types/shelf";
import { ipc } from "../../core/ipc/client";
import { useConfigStore } from "../../core/stores/useConfigStore";

interface FileShelfDrawerProps {
  items: ShelfItem[];
  onBack: () => void;
  onRefresh: () => void;
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

export const FileShelfDrawer: React.FC<FileShelfDrawerProps> = ({
  items,
  onBack,
  onRefresh,
}) => {
  const { config, updateConfig } = useConfigStore();
  const [actionError, setActionError] = useState<string | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const handleCopyPath = async (id: string, filePath: string) => {
    try {
      await navigator.clipboard.writeText(filePath);
      setCopiedId(id);
      setTimeout(() => {
        setCopiedId((prev) => (prev === id ? null : prev));
      }, 1800);
    } catch (err) {
      setActionError("Failed to copy path: " + String(err));
      setTimeout(() => setActionError(null), 3000);
    }
  };

  const handleToggleStorageMode = async () => {
    const nextMode = config.shelfStorageMode === "copy" ? "reference" : "copy";
    try {
      await updateConfig({ ...config, shelfStorageMode: nextMode });
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3000);
    }
  };

  const handleAddFile = async () => {
    setActionError(null);
    setIsProcessing(true);
    try {
      const picked = await ipc.pickShelfFile();
      if (picked) {
        await ipc.addShelfItem(picked);
        onRefresh();
      }
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3500);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleOpen = async (id: string) => {
    setActionError(null);
    try {
      await ipc.openShelfFile(id);
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3500);
    }
  };

  const handleReveal = async (id: string) => {
    setActionError(null);
    try {
      await ipc.showShelfFileInFolder(id);
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3500);
    }
  };

  const handleRemove = async (id: string) => {
    setActionError(null);
    try {
      await ipc.removeShelfItem(id);
      onRefresh();
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3500);
    }
  };

  const handleClear = async () => {
    setActionError(null);
    try {
      await ipc.clearShelf();
      onRefresh();
    } catch (err) {
      setActionError(String(err));
      setTimeout(() => setActionError(null), 3500);
    }
  };

  return (
    <div className="flex flex-col h-full w-full select-none px-1 py-0.5 justify-between">
      {/* Header Bar */}
      <div className="flex items-center justify-between pb-1.5 border-b border-white/10 shrink-0">
        <div className="flex items-center space-x-1.5 min-w-0">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onBack();
            }}
            className="px-2 py-0.5 rounded-md text-[10px] font-semibold orbit-pill-badge-3d text-white/80 hover:text-white transition-colors cursor-pointer flex items-center space-x-1 shrink-0"
            title="Back to Home"
          >
            <span>←</span>
            <span>Back</span>
          </button>
          <span className="text-[11px] font-bold text-white tracking-tight truncate">
            File Shelf
          </span>
          <span className="text-[9.5px] px-1.5 py-0.2 rounded-full bg-blue-500/20 text-blue-300 font-semibold tabular-nums">
            {items.length}/10
          </span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleToggleStorageMode();
            }}
            className={`text-[8.5px] px-1.5 py-0.5 rounded font-semibold transition-colors cursor-pointer border ${
              config.shelfStorageMode === "copy"
                ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/35"
                : "bg-white/5 text-white/60 border-white/10 hover:text-white"
            }`}
            title={
              config.shelfStorageMode === "copy"
                ? "Mode: Managed Copy (copies files ≤50MB into ORBIT storage). Click to switch to Reference mode."
                : "Mode: Reference Only (links original file without copying). Click to switch to Managed Copy mode."
            }
          >
            {config.shelfStorageMode === "copy" ? "Copy ≤50M" : "Ref Mode"}
          </button>
        </div>

        <div className="flex items-center space-x-1.5 shrink-0">
          {items.length < 10 && (
            <button
              type="button"
              disabled={isProcessing}
              onClick={(e) => {
                e.stopPropagation();
                handleAddFile();
              }}
              className="px-2 py-0.5 rounded-md text-[9.5px] font-semibold orbit-pill-badge-3d text-blue-300 hover:text-blue-200 hover:bg-blue-500/20 transition-colors cursor-pointer disabled:opacity-50"
              title="Add file reference to Shelf"
            >
              + Add
            </button>
          )}
          {items.length > 0 && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleClear();
              }}
              className="px-2 py-0.5 rounded-md text-[9.5px] font-medium text-rose-300 hover:text-rose-200 hover:bg-rose-500/20 transition-colors cursor-pointer"
              title="Clear all references from shelf (keeps original files)"
            >
              Clear
            </button>
          )}
        </div>
      </div>

      {/* Error Banner */}
      {actionError && (
        <div className="text-[9.5px] text-rose-300 bg-rose-950/60 border border-rose-800/40 rounded px-2 py-0.5 truncate my-0.5 shrink-0">
          {actionError}
        </div>
      )}

      {/* Items Scrollable List */}
      <div className="flex-1 overflow-y-auto space-y-1 my-1 pr-0.5 min-h-0">
        {items.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center py-4 text-white/40">
            <svg
              className="w-6 h-6 mb-1 text-white/20"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth="1.5"
            >
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            <span className="text-[10.5px] font-medium">Shelf is empty</span>
            <span className="text-[9px] text-white/30 mt-0.5">
              Drag &amp; drop files here or click + Add
            </span>
          </div>
        ) : (
          items.map((item) => {
            const ext = (item.extension || "file").toUpperCase().slice(0, 4);
            const isMissing = item.exists === false;
            return (
              <div
                key={item.id}
                className={`orbit-well-3d rounded-lg px-2 py-1 flex items-center justify-between text-left group hover:bg-white/[0.04] transition-colors ${
                  isMissing ? "border border-rose-500/30 opacity-80" : ""
                }`}
              >
                {/* File Icon Badge + Name + Size */}
                <div className="flex items-center space-x-2 min-w-0 flex-1 mr-2">
                  <span
                    className={`text-[8px] font-mono font-bold px-1.5 py-0.5 rounded uppercase shrink-0 ${
                      isMissing
                        ? "bg-rose-500/20 text-rose-300 border border-rose-500/30"
                        : "bg-blue-500/20 text-blue-400 border border-blue-500/30"
                    }`}
                  >
                    {ext}
                  </span>
                  <div className="min-w-0 flex-1 leading-tight">
                    <div className="flex items-center space-x-1 min-w-0">
                      <span
                        className={`text-[10.5px] font-medium truncate ${
                          isMissing ? "text-rose-200 line-through" : "text-white"
                        }`}
                        title={item.filePath}
                      >
                        {item.displayName}
                      </span>
                      {isMissing && (
                        <span
                          className="text-[7.5px] px-1 py-0.2 rounded bg-rose-500/25 text-rose-300 border border-rose-500/40 font-bold uppercase shrink-0"
                          title="File was moved or deleted from its original location"
                        >
                          Missing
                        </span>
                      )}
                      {!isMissing && item.isManagedCopy && (
                        <span
                          className="text-[7.5px] px-1 py-0.2 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-semibold uppercase shrink-0"
                          title="Stored in ORBIT managed shelf storage"
                        >
                          Copy
                        </span>
                      )}
                    </div>
                    <div className="text-[8.5px] text-white/40 font-mono">
                      {formatFileSize(item.fileSize)}
                    </div>
                  </div>
                </div>

                {/* Actions: Copy Path, Open, Reveal, Remove */}
                <div className="flex items-center space-x-1 shrink-0">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCopyPath(item.id, item.filePath);
                    }}
                    className={`p-1 rounded transition-colors cursor-pointer ${
                      copiedId === item.id
                        ? "text-emerald-300 bg-emerald-500/20"
                        : "text-white/60 hover:text-white hover:bg-white/10"
                    }`}
                    title={
                      copiedId === item.id
                        ? "Copied path to clipboard!"
                        : `Copy path: ${item.filePath}`
                    }
                  >
                    {copiedId === item.id ? (
                      <svg
                        className="w-3 h-3"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2.5"
                      >
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                    ) : (
                      <svg
                        className="w-3 h-3"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                      >
                        <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                      </svg>
                    )}
                  </button>

                  <button
                    type="button"
                    disabled={isMissing}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleOpen(item.id);
                    }}
                    className="p-1 rounded text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                    title={isMissing ? "File missing at original path" : "Open file"}
                  >
                    <svg
                      className="w-3 h-3"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                    >
                      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                      <polyline points="15 3 21 3 21 9" />
                      <line x1="10" y1="14" x2="21" y2="3" />
                    </svg>
                  </button>

                  <button
                    type="button"
                    disabled={isMissing}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleReveal(item.id);
                    }}
                    className="p-1 rounded text-white/60 hover:text-white hover:bg-white/10 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                    title={isMissing ? "File missing at original path" : "Reveal in File Explorer"}
                  >
                    <svg
                      className="w-3 h-3"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                    >
                      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                    </svg>
                  </button>

                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemove(item.id);
                    }}
                    className="p-1 rounded text-white/40 hover:text-rose-300 hover:bg-rose-500/20 transition-colors cursor-pointer"
                    title="Remove from Shelf (never deletes original user file)"
                  >
                    <svg
                      className="w-3 h-3"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                    >
                      <line x1="18" y1="6" x2="6" y2="18" />
                      <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Footer Hint */}
      <div className="text-[8.5px] text-white/35 flex items-center justify-between pt-0.5 border-t border-white/5 shrink-0">
        <span>References preserved across restarts</span>
        <span>Original files stay intact</span>
      </div>
    </div>
  );
};
