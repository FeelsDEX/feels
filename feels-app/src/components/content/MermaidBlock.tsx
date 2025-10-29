'use client';

import { useEffect, useRef, useState } from 'react';
import { useLightbox } from '@/hooks/useLightbox';

interface MermaidBlockProps {
  chart: string;
}

export function MermaidBlock({ chart }: MermaidBlockProps) {
  const elementRef = useRef<HTMLDivElement>(null);
  const [mermaid, setMermaid] = useState<any>(null);
  const [isInitialized, setIsInitialized] = useState(false);
  const { openLightbox } = useLightbox();

  useEffect(() => {
    // Dynamic import of mermaid only on client side
    if (typeof window !== 'undefined') {
      import('mermaid').then((mermaidModule) => {
        const mermaidInstance = mermaidModule.default;
        setMermaid(mermaidInstance);
        
        // Initialize mermaid with our theme
        if (!isInitialized) {
          mermaidInstance.initialize({
            startOnLoad: true,
            theme: 'default',
            themeVariables: {
              primaryColor: '#ffffff',
              primaryTextColor: '#000',
              primaryBorderColor: '#e5e5e5',
              lineColor: '#666',
              secondaryColor: '#f5f5f5',
              background: 'transparent',
              mainBkg: '#ffffff',
              secondBkg: '#f5f5f5',
              tertiaryColor: '#ffffff',
              fontSize: '16px',
              nodeBkg: '#ffffff',
              nodeTextColor: '#000',
            },
            flowchart: {
              htmlLabels: true,
              curve: 'linear',
              rankSpacing: 80,
              nodeSpacing: 80,
              padding: 20,
              useMaxWidth: true,
            },
            sequence: {
              diagramMarginX: 50,
              diagramMarginY: 30,
              actorMargin: 100,
              width: 200,
              height: 65,
              boxMargin: 20,
              boxTextMargin: 5,
              noteMargin: 15,
              messageMargin: 45,
              mirrorActors: true,
            },
            // Ensure proper text centering for all diagram types
            class: {
              useMaxWidth: true,
            },
            state: {
              useMaxWidth: true,
            },
            gantt: {
              useMaxWidth: true,
            },
            journey: {
              useMaxWidth: true,
            },
          });
          setIsInitialized(true);
        }
      });
    }
  }, [isInitialized]);

  useEffect(() => {
    if (!mermaid || typeof window === 'undefined' || !elementRef.current) return;

    const renderDiagram = async () => {
      if (elementRef.current) {
        try {
          elementRef.current.innerHTML = '';
          const id = `mermaid-${Date.now()}`;
          const { svg } = await mermaid.render(id, chart);
          elementRef.current.innerHTML = svg;
          
          // Post-render: Force text centering for all text elements and add click handler
          setTimeout(() => {
            const textElements = elementRef.current?.querySelectorAll('text');
            textElements?.forEach((text) => {
              text.setAttribute('text-anchor', 'middle');
              text.style.textAnchor = 'middle';
              text.setAttribute('dominant-baseline', 'central');
              text.style.dominantBaseline = 'central';
            });

            // Add click handler for lightbox
            if (elementRef.current) {
              const svg = elementRef.current.querySelector('svg');
              if (svg) {
                elementRef.current.style.cursor = 'pointer';
                elementRef.current.style.position = 'relative';
                elementRef.current.title = 'Click to view full size';
                
                // Create lightbox icon
                const icon = document.createElement('div');
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
                icon.style.zIndex = '10';
                
                elementRef.current.appendChild(icon);
                
                // Show/hide icon on hover
                elementRef.current.addEventListener('mouseenter', () => {
                  icon.style.opacity = '1';
                });
                
                elementRef.current.addEventListener('mouseleave', () => {
                  icon.style.opacity = '0';
                });
                
                elementRef.current.addEventListener('click', () => {
                  openLightbox({
                    type: 'mermaid',
                    element: elementRef.current!,
                    title: 'Mermaid Diagram'
                  });
                });
              }
            }
          }, 100);
        } catch (error) {
          console.error('Failed to render mermaid diagram:', error);
          if (elementRef.current) {
            elementRef.current.innerHTML = `<pre class="text-danger-500">Failed to render diagram: ${error}</pre>`;
          }
        }
      }
    };

    renderDiagram();
  }, [chart, mermaid, openLightbox]);

  return (
    <div className="mermaid-container my-8 overflow-x-auto">
      <div ref={elementRef} className="mermaid" />
    </div>
  );
}