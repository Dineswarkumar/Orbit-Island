import { AppConfig, CompactRightSlot } from "../types/config";

/** Smart Auto uses only opted-in online weather; custom labels require explicit selection. */
export function resolveCompactRightSlot(
  config: Pick<AppConfig, "compactRightSlot" | "onlineWeatherEnabled">,
  hasWeather: boolean,
): CompactRightSlot {
  if (config.compactRightSlot !== "smart") return config.compactRightSlot;
  return config.onlineWeatherEnabled && hasWeather ? "weather" : "orbit_logo";
}
