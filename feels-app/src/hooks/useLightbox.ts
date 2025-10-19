'use client';

import { createContext, useContext, useState } from 'react';

interface LightboxContent {
  type: 'image' | 'mermaid' | 'video';
  src?: string;
  alt?: string;
  element?: HTMLElement;
  title?: string;
}

interface LightboxContextType {
  isOpen: boolean;
  content: LightboxContent | null;
  openLightbox: (content: LightboxContent) => void;
  closeLightbox: () => void;
}

const LightboxContext = createContext<LightboxContextType | undefined>(undefined);

export function useLightbox() {
  const context = useContext(LightboxContext);
  if (!context) {
    throw new Error('useLightbox must be used within a LightboxProvider');
  }
  return context;
}

export function useLightboxState() {
  const [isOpen, setIsOpen] = useState(false);
  const [content, setContent] = useState<LightboxContent | null>(null);

  const openLightbox = (newContent: LightboxContent) => {
    setContent(newContent);
    setIsOpen(true);
  };

  const closeLightbox = () => {
    setIsOpen(false);
    // Delay clearing content to allow for exit animations
    setTimeout(() => setContent(null), 300);
  };

  return {
    isOpen,
    content,
    openLightbox,
    closeLightbox,
  };
}

export { LightboxContext };