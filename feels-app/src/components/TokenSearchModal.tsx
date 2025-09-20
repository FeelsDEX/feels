'use client';

import { useEffect } from 'react';
import { TokenSearchResult } from '@/lib/token-search';
import { SearchBar } from '@/components/SearchBar';
import { useSearchContext } from '@/contexts/SearchContext';

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

  if (!isOpen) return null;

  return (
    <div id="token-search-modal-container" className="fixed top-0 left-0 right-0 z-[1100] pt-2">
      <div id="token-search-modal-inner" className="container mx-auto px-4">
        <div id="token-search-modal-flex" className="flex items-center h-16">
          {/* Spacer for logo area */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div id="token-search-modal-search-wrapper" className="flex-1 max-w-xl mx-8 relative z-[1101]">
            <SearchBar
              mode="token-select"
              placeholder={placeholder}
              onTokenSelect={onSelect}
              excludeAddress={excludeAddress}
              onClose={onClose}
              autoFocus={true}
            />
          </div>
          
          {/* Spacer for right side */}
          <div className="flex-1" />
        </div>
      </div>
    </div>
  );
}