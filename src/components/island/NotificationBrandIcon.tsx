import React from "react";
import { NotificationItem } from "../../core/types/notifications";

interface NotificationBrandIconProps {
  notification: NotificationItem;
  size?: "compact" | "inspector";
  fallbackColor?: string;
}

export const NotificationBrandIcon: React.FC<NotificationBrandIconProps> = ({
  notification,
  size = "compact",
  fallbackColor = "#60a5fa",
}) => {
  const isCompact = size === "compact";
  const sizeClass = isCompact ? "w-10 h-10" : "w-9 h-9";
  const iconSizeClass = isCompact ? "w-5 h-5" : "w-[18px] h-[18px]";

  const probe = `${notification.appName} ${notification.appId} ${notification.title} ${notification.body}`.toLowerCase();

  // 1. WhatsApp
  if (probe.includes("whatsapp")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#25D366" }}
        title="WhatsApp"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M12.04 2C6.58 2 2.13 6.45 2.13 11.91C2.13 13.66 2.59 15.36 3.45 16.86L2.05 22L7.3 20.62C8.75 21.41 10.38 21.83 12.04 21.83C17.5 21.83 21.95 17.38 21.95 11.92C21.95 9.27 20.92 6.78 19.05 4.91C17.18 3.03 14.69 2 12.04 2M12.05 3.67C14.25 3.67 16.31 4.53 17.87 6.09C19.42 7.65 20.28 9.72 20.28 11.92C20.28 16.46 16.58 20.15 12.04 20.15C10.56 20.15 9.11 19.76 7.85 19L7.55 18.83L4.43 19.65L5.26 16.61L5.06 16.29C4.24 14.99 3.8 13.47 3.8 11.91C3.81 7.37 7.5 3.67 12.05 3.67M9.53 7.34C9.36 7.34 9.08 7.4 8.84 7.66C8.6 7.92 7.93 8.55 7.93 9.83C7.93 11.11 8.87 12.35 9 12.53C9.13 12.7 10.84 15.33 13.45 16.46C14.07 16.73 14.56 16.89 14.94 17.01C15.56 17.21 16.12 17.18 16.57 17.11C17.07 17.04 18.1 16.49 18.31 15.89C18.53 15.29 18.53 14.78 18.46 14.67C18.4 14.56 18.23 14.49 17.97 14.36C17.71 14.23 16.44 13.6 16.2 13.51C15.97 13.43 15.8 13.39 15.63 13.64C15.46 13.9 14.98 14.49 14.83 14.67C14.68 14.84 14.53 14.86 14.28 14.73C14.02 14.6 12.94 14.25 11.66 13.11C10.66 12.22 9.99 11.12 9.86 10.91C9.74 10.69 9.85 10.58 9.98 10.45C10.1 10.33 10.24 10.14 10.37 9.99C10.5 9.85 10.54 9.74 10.63 9.57C10.71 9.4 10.67 9.25 10.61 9.12C10.55 9 10.05 7.77 9.85 7.27C9.64 6.78 9.44 6.84 9.29 6.84L8.84 6.83C8.6 6.83 8.35 6.9 8.18 7.07C8.01 7.24 7.54 7.68 7.54 8.58C7.54 9.48 8.19 10.35 8.28 10.47L9.53 7.34Z" />
        </svg>
      </div>
    );
  }

  // 2. Telegram
  if (probe.includes("telegram") || probe.includes("t.me")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#2AABEE" }}
        title="Telegram"
      >
        <svg
          className={`${iconSizeClass} text-white translate-x-[-0.5px]`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm4.64 6.8c-.15 1.58-.8 5.42-1.13 7.19-.14.75-.42 1-.68 1.03-.58.05-1.02-.38-1.58-.75-.88-.58-1.38-.94-2.23-1.5-.99-.65-.35-1.01.22-1.59.15-.15 2.71-2.48 2.76-2.69a.2.2 0 0 0-.05-.18c-.06-.05-.14-.03-.21-.02-.09.02-1.49.95-4.22 2.79-.4.27-.76.41-1.08.4-.36-.01-1.04-.2-1.55-.37-.63-.2-1.12-.31-1.08-.66.02-.18.27-.36.74-.55 2.92-1.27 4.86-2.11 5.83-2.51 2.78-1.16 3.35-1.36 3.73-1.36.08 0 .27.02.39.12.1.08.13.19.14.27-.01.06.01.24 0 .38z" />
        </svg>
      </div>
    );
  }

  // 3. Discord
  if (probe.includes("discord")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#5865F2" }}
        title="Discord"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.894.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
        </svg>
      </div>
    );
  }

  // 4. Gmail / Google Mail / Outlook / Email
  if (
    probe.includes("gmail") ||
    probe.includes("mail.google") ||
    probe.includes("outlook") ||
    probe.includes("mail")
  ) {
    const isOutlook = probe.includes("outlook");
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: isOutlook ? "#0078D4" : "#EA4335" }}
        title={isOutlook ? "Outlook" : "Gmail"}
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M20 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 4l-8 5-8-5V6l8 5 8-5v2z" />
        </svg>
      </div>
    );
  }

  // 5. Instagram
  if (probe.includes("instagram")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{
          background:
            "radial-gradient(circle at 30% 107%, #fdf497 0%, #fdf497 5%, #fd5949 45%, #d6249f 60%, #285AEB 90%)",
        }}
        title="Instagram"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z" />
        </svg>
      </div>
    );
  }

  // 6. YouTube
  if (probe.includes("youtube")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#FF0000" }}
        title="YouTube"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z" />
        </svg>
      </div>
    );
  }

  // 7. Slack
  if (probe.includes("slack")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#4A154B" }}
        title="Slack"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zM8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zM18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zM15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z" />
        </svg>
      </div>
    );
  }

  // 8. GitHub
  if (probe.includes("github")) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md`}
        style={{ backgroundColor: "#24292F" }}
        title="GitHub"
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12.017C22 6.484 17.522 2 12 2z" />
        </svg>
      </div>
    );
  }

  // 9. Web Browser (Chrome / Edge / Brave / Firefox)
  if (
    probe.includes("chrome") ||
    probe.includes("msedge") ||
    probe.includes("edge") ||
    probe.includes("brave") ||
    probe.includes("firefox")
  ) {
    return (
      <div
        className={`${sizeClass} rounded-xl flex items-center justify-center shrink-0 shadow-md bg-gradient-to-br from-blue-500 to-indigo-600`}
        title={notification.appName}
      >
        <svg
          className={`${iconSizeClass} text-white`}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="2" y1="12" x2="22" y2="12" />
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>
      </div>
    );
  }

  // 10. Native app icon extracted by WinRT. These bitmaps ship with generous transparent
  // padding, so render them unframed (no tile/border) and scaled up so the logo reads clearly.
  if (notification.appIcon) {
    return (
      <div
        className={`${sizeClass} shrink-0 flex items-center justify-center overflow-visible`}
        title={notification.appName}
      >
        <img
          src={notification.appIcon}
          alt={notification.appName}
          draggable={false}
          className="w-full h-full object-contain scale-[1.3] drop-shadow-[0_2px_4px_rgba(0,0,0,0.45)]"
        />
      </div>
    );
  }

  // 11. Generic Theme Fallback Bell
  return (
    <div
      className={`${sizeClass} rounded-xl overflow-hidden shrink-0 shadow-md flex items-center justify-center bg-white/10 border border-white/15`}
    >
      <svg
        className={`${iconSizeClass} drop-shadow-sm`}
        style={{ color: fallbackColor }}
        viewBox="0 0 24 24"
        fill="currentColor"
      >
        <path d="M12 2a6 6 0 0 0-6 6v3.586l-.707.707A1 1 0 0 0 6 14h12a1 1 0 0 0 .707-1.707L18 11.586V8a6 6 0 0 0-6-6zm0 18a2.5 2.5 0 0 0 2.45-2h-4.9A2.5 2.5 0 0 0 12 20z" />
      </svg>
    </div>
  );
};
