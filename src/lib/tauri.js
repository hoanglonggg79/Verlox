import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";

export const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invoke(command, args) {
  return tauriInvoke(command, args);
}

export function listen(event, handler) {
  return tauriListen(event, handler);
}
