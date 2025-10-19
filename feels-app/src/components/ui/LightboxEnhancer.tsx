'use client';

import { useEffect } from 'react';
import { useLightbox } from '@/hooks/useLightbox';

interface LightboxEnhancerProps {
  selector?: string;
}

export function LightboxEnhancer({ selector = '.prose' }: LightboxEnhancerProps) {
  const { openLightbox } = useLightbox();

  useEffect(() => {
    // Add a small delay to ensure DOM is fully loaded
    const timer = setTimeout(() => {
      const container = document.querySelector(selector);
      if (!container) return;

      // Add click handlers to images
      const images = container.querySelectorAll('img:not([data-lightbox-disabled])');
    
      images.forEach((img) => {
      const imageElement = img as HTMLImageElement;
      
      // Skip if already has lightbox
      if (imageElement.dataset['lightboxEnabled'] === 'true') return;
      
      // Create wrapper div for relative positioning
      const wrapper = document.createElement('div');
      wrapper.style.position = 'relative';
      wrapper.style.display = 'inline-block';
      wrapper.style.cursor = 'pointer';
      
      // Create lightbox icon
      const icon = document.createElement('div');
      icon.className = 'lightbox-icon';
      icon.innerHTML = `
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 3H5a2 2 0 0 0-2 2v3"/>
          <path d="M21 8V5a2 2 0 0 0-2-2h-3"/>
          <path d="M3 16v3a2 2 0 0 0 2 2h3"/>
          <path d="M16 21h3a2 2 0 0 0 2-2v-3"/>
        </svg>
      `;
      icon.style.position = 'absolute';
      icon.style.bottom = '8px';
      icon.style.right = '8px';
      icon.style.width = '20px';
      icon.style.height = '20px';
      icon.style.background = 'rgba(128, 128, 128, 0.8)';
      icon.style.borderRadius = '4px';
      icon.style.border = '1px solid rgba(255, 255, 255, 0.2)';
      icon.style.display = 'flex';
      icon.style.alignItems = 'center';
      icon.style.justifyContent = 'center';
      icon.style.opacity = '0';
      icon.style.transition = 'opacity 0.2s ease-out';
      icon.style.pointerEvents = 'none';
      
      // Wrap the image
      imageElement.parentNode?.insertBefore(wrapper, imageElement);
      wrapper.appendChild(imageElement);
      wrapper.appendChild(icon);
      
      imageElement.title = imageElement.title || 'Click to view full size';
      imageElement.dataset['lightboxEnabled'] = 'true';
      
      // Show/hide icon on hover
      wrapper.addEventListener('mouseenter', () => {
        icon.style.opacity = '1';
      });
      
      wrapper.addEventListener('mouseleave', () => {
        icon.style.opacity = '0';
      });
      
      const handleClick = () => {
        openLightbox({
          type: 'image',
          src: imageElement.src,
          alt: imageElement.alt,
          title: imageElement.alt || 'Image'
        });
      };
      
      wrapper.addEventListener('click', handleClick);
      
      // Store cleanup function
      (imageElement as any)._lightboxCleanup = () => {
        wrapper.removeEventListener('click', handleClick);
        wrapper.removeEventListener('mouseenter', () => {});
        wrapper.removeEventListener('mouseleave', () => {});
      };
    });

    // Add click handlers to videos
    const videos = container.querySelectorAll('video:not([data-lightbox-disabled])');
    videos.forEach((video) => {
      const videoElement = video as HTMLVideoElement;
      
      // Skip if already has lightbox
      if (videoElement.dataset['lightboxEnabled'] === 'true') return;
      
      // Add a wrapper with click handler for fullscreen
      const wrapper = document.createElement('div');
      wrapper.style.position = 'relative';
      wrapper.style.cursor = 'pointer';
      wrapper.style.display = 'inline-block';
      wrapper.title = 'Click to view full size';
      
      videoElement.parentNode?.insertBefore(wrapper, videoElement);
      wrapper.appendChild(videoElement);
      
      videoElement.dataset['lightboxEnabled'] = 'true';
      
      const handleClick = (e: Event) => {
        // Only trigger if not clicking on video controls
        const target = e.target as HTMLElement;
        if (target.tagName !== 'VIDEO') return;
        
        openLightbox({
          type: 'video',
          src: videoElement.src,
          title: 'Video'
        });
      };
      
      wrapper.addEventListener('click', handleClick);
      
      // Store cleanup function
      (videoElement as any)._lightboxCleanup = () => {
        wrapper.removeEventListener('click', handleClick);
      };
    });

    // Cleanup function
    return () => {
      [...images, ...videos].forEach((element) => {
        if ((element as any)._lightboxCleanup) {
          (element as any)._lightboxCleanup();
        }
      });
    };
    }, 500); // 500ms delay

    return () => clearTimeout(timer);
  }, [selector, openLightbox]);

  return null; // This component doesn't render anything
}