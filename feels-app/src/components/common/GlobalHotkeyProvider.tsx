'use client';

import { useGlobalSearchHotkey } from '@/hooks/useGlobalSearchHotkey';

export function GlobalHotkeyProvider({ children }: { children: React.ReactNode }) {
  useGlobalSearchHotkey();
  return <>{children}</>;
}