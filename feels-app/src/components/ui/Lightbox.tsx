'use client';

import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';

interface LightboxProps {
  isOpen: boolean;
  onClose: () => void;
  content: {
    type: 'image' | 'mermaid' | 'video';
    src?: string;
    alt?: string;
    element?: HTMLElement;
    title?: string;
  };
}

export function Lightbox({ isOpen, onClose, content }: LightboxProps) {
  const overlayRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const mermaidRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = 'unset';
    };
  }, [isOpen, onClose]);

  // Handle Mermaid diagram rendering
  useEffect(() => {
    if (content.type === 'mermaid' && content.element && mermaidRef.current && isOpen) {
      const svg = content.element.querySelector('svg');
      
      if (svg) {
        // Clear any existing content
        mermaidRef.current.innerHTML = '';
        
        // Create a wrapper div to center the SVG
        const wrapper = document.createElement('div');
        wrapper.style.display = 'flex';
        wrapper.style.alignItems = 'center';
        wrapper.style.justifyContent = 'center';
        wrapper.style.width = '100%';
        wrapper.style.height = '100%';
        wrapper.style.minHeight = '60vh';
        
        // Copy the SVG
        const svgOuterHTML = svg.outerHTML;
        wrapper.innerHTML = svgOuterHTML;
        
        // Append wrapper to container
        mermaidRef.current.appendChild(wrapper);
        
        // Get the newly inserted SVG and ensure it displays properly
        const newSvg = wrapper.querySelector('svg');
        if (newSvg) {
          // Ensure the SVG fits within viewport while maintaining aspect ratio
          newSvg.style.display = 'block';
          newSvg.style.width = 'auto';
          newSvg.style.height = 'auto';
          newSvg.style.maxWidth = 'calc(100vw - 2rem)';
          newSvg.style.maxHeight = 'calc(100vh - 2rem)';
          newSvg.style.objectFit = 'contain';
          newSvg.style.position = 'static';
          newSvg.style.transform = 'none';
          newSvg.style.left = 'auto';
          newSvg.style.top = 'auto';
          newSvg.style.right = 'auto';
          newSvg.style.bottom = 'auto';
          
          // Ensure all paths and elements are visible
          const paths = newSvg.querySelectorAll('path, rect, circle, ellipse, line, polyline, polygon, text');
          paths.forEach(element => {
            const el = element as SVGElement;
            if (el.style.visibility === 'hidden') {
              el.style.visibility = 'visible';
            }
            if (el.style.opacity === '0') {
              el.style.opacity = '1';
            }
          });
        }
      }
    }
  }, [content, isOpen]);

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === overlayRef.current) {
      onClose();
    }
  };


  if (!isOpen) return null;

  return (
    <div
      ref={overlayRef}
      className="lightbox-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-sm"
      onClick={handleOverlayClick}
    >
      {/* Header with controls */}
      <div className="absolute top-4 right-4 flex items-center gap-2 z-10">
        <button
          onClick={onClose}
          className="text-white hover:text-gray-300 rounded transition-colors"
          style={{ 
            backgroundColor: 'rgba(128, 128, 128, 0.8)',
            padding: '4px'
          }}
          title="Close (Esc)"
        >
          <X size={20} />
        </button>
      </div>

      {/* Content */}
      <div
        ref={contentRef}
        className={`lightbox-content ${
          content.type === 'mermaid' ? 'w-full h-full pt-16' : 'max-w-[90vw] max-h-[90vh] p-4 mt-16 mb-4'
        }`}
      >
        {content.type === 'image' && content.src && (
          <img
            src={content.src}
            alt={content.alt}
            className="max-w-full max-h-full object-contain mx-auto rounded-lg shadow-2xl"
          />
        )}
        
        {content.type === 'mermaid' && content.element && (
          <div className="mermaid-lightbox bg-white w-full h-full absolute inset-0 p-4">
            <div 
              ref={mermaidRef} 
              className="mermaid-lightbox-content w-full h-full"
            />
          </div>
        )}
        
        {content.type === 'video' && content.src && (
          <video
            src={content.src}
            controls
            className="max-w-full max-h-full mx-auto rounded-lg shadow-2xl"
          />
        )}
      </div>
    </div>
  );
}