'use client';

import React, { ReactNode } from 'react';
import { LightboxContext, useLightboxState } from '@/hooks/useLightbox';
import { Lightbox } from './Lightbox';

interface LightboxProviderProps {
  children: ReactNode;
}

export function LightboxProvider({ children }: LightboxProviderProps) {
  const lightboxState = useLightboxState();

  return (
    <LightboxContext.Provider value={lightboxState}>
      {children}
      <Lightbox
        isOpen={lightboxState.isOpen}
        onClose={lightboxState.closeLightbox}
        content={lightboxState.content || { type: 'image' }}
      />
    </LightboxContext.Provider>
  );
}