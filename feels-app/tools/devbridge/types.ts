// Bridge message protocol types
export type BridgeMsg =
  | { t: "log"; level: "log" | "warn" | "error"; ts: number; origin: "browser" | "server"; msg: unknown[] }
  | { t: "event"; name: string; ts: number; data?: unknown }
  | { t: "command"; id: string; name: string; args?: unknown }
  | { t: "result"; id: string; ok: boolean; data?: unknown; error?: string }
  | { t: "hello"; role: "cli" | "app"; version: 1 };

export interface ClientSocket {
  id: string;
  role: "cli" | "app";
  socket: any;
}