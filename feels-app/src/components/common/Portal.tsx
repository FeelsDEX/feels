'use client';

import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';

interface PortalProps {
  children: React.ReactNode;
}

export function Portal({ children }: PortalProps) {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  return mounted && typeof document !== 'undefined' 
    ? createPortal(children, document.body) 
    : null;
}