'use client';

import { useMemo, useEffect } from 'react';
import { MermaidBlock } from './MermaidBlock';
import { LightboxEnhancer } from '../ui/LightboxEnhancer';

interface ProcessedContentProps {
  content: string;
  className?: string;
}

export function ProcessedContent({ content, className }: ProcessedContentProps) {
  const processedElements = useMemo(() => {
    // Split content by mermaid code blocks
    const mermaidRegex = /<pre[^>]*><code[^>]*data-language="mermaid"[^>]*>([\s\S]*?)<\/code><\/pre>/g;
    const parts: (string | { type: 'mermaid'; content: string })[] = [];
    let lastIndex = 0;
    let match;

    while ((match = mermaidRegex.exec(content)) !== null) {
      // Add HTML before the mermaid block
      if (match.index > lastIndex) {
        parts.push(content.slice(lastIndex, match.index));
      }

      // Add mermaid block - decode HTML entities
      let mermaidContent = match[1] || '';
      
      // Create a temporary element to decode HTML entities
      const temp = document.createElement('div');
      temp.innerHTML = mermaidContent;
      mermaidContent = temp.textContent || temp.innerText || '';

      parts.push({ type: 'mermaid', content: mermaidContent });
      lastIndex = match.index + match[0].length;
    }

    // Add remaining HTML
    if (lastIndex < content.length) {
      parts.push(content.slice(lastIndex));
    }

    return parts;
  }, [content]);

  // Add anchor link functionality to headings and identify standalone math equations
  useEffect(() => {
    // Simple logic: add class to paragraphs that contain only math
    const paragraphs = document.querySelectorAll('.prose p');
    paragraphs.forEach((p) => {
      if (p instanceof HTMLElement) {
        const katexElements = p.querySelectorAll('.katex');
        
        // Check if paragraph has only KaTeX and no significant text nodes
        let hasSignificantText = false;
        p.childNodes.forEach(node => {
          if (node.nodeType === Node.TEXT_NODE) {
            const text = node.textContent?.trim() || '';
            if (text.length > 0) {
              hasSignificantText = true;
            }
          }
        });
        
        // Only center if: has exactly 1 KaTeX element AND has exactly 1 child element AND no text nodes
        if (katexElements.length === 1 && p.children.length === 1 && !hasSignificantText) {
          p.classList.add('math-equation-only');
        }
      }
    });
    
    // Handle any math-like content
    const dollarElements = document.querySelectorAll('*');
    dollarElements.forEach(el => {
      if (el.textContent?.includes('$$') || el.innerHTML?.includes('$$')) {
        if (el instanceof HTMLElement) {
          el.style.textAlign = 'center';
          el.style.display = 'block';
          el.style.margin = '1rem auto';
        }
      }
    });

    // Only select headings within prose content, excluding navbar
    const headings = document.querySelectorAll('.prose h1[id], .prose h2[id], .prose h3[id], .prose h4[id], .prose h5[id], .prose h6[id]');
    
    headings.forEach((heading) => {
      const id = heading.id;
      if (!id) return;
      
      // Create anchor link
      const anchor = document.createElement('a');
      anchor.href = `#${id}`;
      anchor.className = 'anchor-link';
      anchor.setAttribute('aria-label', 'Link to this section');
      // Use Lucide link-2 icon SVG
      anchor.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 17H7A5 5 0 0 1 7 7h2"></path><path d="M15 7h2a5 5 0 1 1 0 10h-2"></path><line x1="8" x2="16" y1="12" y2="12"></line></svg>';
      
      // Style the anchor based on heading level
      const tagName = heading.tagName.toLowerCase();
      
      // Add appropriate spacing to the heading based on its level
      if (heading instanceof HTMLElement) {
        heading.style.position = 'relative';
        
        if (tagName === 'h1') {
          heading.style.paddingLeft = '2rem';
          heading.style.marginLeft = '-2rem';
        } else if (tagName === 'h2') {
          heading.style.paddingLeft = '1.75rem';
          heading.style.marginLeft = '-1.75rem';
        } else {
          heading.style.paddingLeft = '1.5rem';
          heading.style.marginLeft = '-1.5rem';
        }
      }
      
      if (anchor instanceof HTMLElement) {
        anchor.style.position = 'absolute';
        anchor.style.left = '0';
        anchor.style.top = '50%';
        anchor.style.transform = 'translateY(-50%)';
        anchor.style.opacity = '0';
        anchor.style.transition = 'opacity 0.2s';
        anchor.style.textDecoration = 'none';
        anchor.style.display = 'inline-flex';
        anchor.style.alignItems = 'center';
        
        // Add hover effects
        heading.addEventListener('mouseenter', () => {
          if (anchor instanceof HTMLElement) {
            anchor.style.opacity = '1';
          }
        });
        
        heading.addEventListener('mouseleave', () => {
          if (anchor instanceof HTMLElement) {
            anchor.style.opacity = '0';
          }
        });
      }
      
      // Add click to copy behavior
      anchor.addEventListener('click', (e) => {
        e.preventDefault();
        const url = `${window.location.origin}${window.location.pathname}#${id}`;
        navigator.clipboard.writeText(url);
        window.location.hash = id;
      });
      
      // Insert anchor at the beginning of the heading
      heading.insertBefore(anchor, heading.firstChild);
    });
    
    // Cleanup
    return () => {
      const anchors = document.querySelectorAll('.anchor-link');
      anchors.forEach(anchor => anchor.remove());
    };
  }, [content]);

  return (
    <div className={className}>
      <LightboxEnhancer selector=".prose" />
      {processedElements.map((part, index) => {
        if (typeof part === 'string') {
          return <div key={index} dangerouslySetInnerHTML={{ __html: part }} />;
        } else {
          return <MermaidBlock key={index} chart={part.content} />;
        }
      })}
    </div>
  );
}