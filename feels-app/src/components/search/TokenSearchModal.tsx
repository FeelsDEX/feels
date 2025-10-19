'use client';

import { useEffect } from 'react';
import { TokenSearchResult } from '@/utils/token-search';
import { SearchBar } from '@/components/search/SearchBar';
import { useSearchContext } from '@/contexts/SearchContext';
import { Portal } from '@/components/common/Portal';
import { useTokenSearch } from '@/hooks/useTokenSearch';

interface TokenSearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (token: TokenSearchResult) => void;
  excludeAddress?: string;
  placeholder?: string;
}

export function TokenSearchModal({ 
  isOpen, 
  onClose, 
  onSelect,
  excludeAddress,
  placeholder = "Find tokens by name, ticker, or address..."
}: TokenSearchModalProps) {
  const { setIsTokenSearchModalOpen } = useSearchContext();
  
  // Pre-load popular tokens for instant display
  const { results: preloadedTokens } = useTokenSearch('');

  // Update context when modal opens/closes
  useEffect(() => {
    setIsTokenSearchModalOpen(isOpen);
    return () => {
      setIsTokenSearchModalOpen(false);
    };
  }, [isOpen, setIsTokenSearchModalOpen]);

  // Close on escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // Handle backdrop click to close modal
  const handleBackdropClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <Portal>
      {/* Full screen backdrop to capture all clicks */}
      <div 
        className="fixed inset-0 z-[2000] bg-background/80 backdrop-blur-sm" 
        onClick={handleBackdropClick}
      />
      <div id="token-search-modal-container" className="fixed top-0 left-0 right-0 z-[2001] pt-2">
        <div id="token-search-modal-inner" className="container mx-auto px-4">
          <div id="token-search-modal-flex" className="flex items-center h-16">
          {/* Spacer for logo area */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div id="token-search-modal-search-wrapper" className="flex-1 max-w-xl mx-8 relative z-[2002]">
            <SearchBar
              mode="token-select"
              placeholder={placeholder}
              onTokenSelect={onSelect}
              excludeAddress={excludeAddress}
              onClose={onClose}
              autoFocus={true}
              preloadedTokens={preloadedTokens}
            />
          </div>
          
          {/* Spacer for right side */}
          <div className="flex-1" />
        </div>
      </div>
    </div>
    </Portal>
  );
}