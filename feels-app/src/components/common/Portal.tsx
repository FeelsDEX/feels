'use client';

import { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';

interface PortalProps {
  children: React.ReactNode;
}

export function Portal({ children }: PortalProps) {
  const mountRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const mount = document.createElement('div');
    mount.setAttribute('id', 'portal-root');
    document.body.appendChild(mount);
    mountRef.current = mount;

    return () => {
      if (mountRef.current) {
        document.body.removeChild(mountRef.current);
      }
    };
  }, []);

  return mountRef.current ? createPortal(children, mountRef.current) : null;
}