'use client';

import { useEffect } from 'react';
import { SearchBar } from '@/components/search/SearchBar';
import { useSearchContext } from '@/contexts/SearchContext';
import { Portal } from '@/components/common/Portal';

interface SearchPageModalProps {
  isOpen: boolean;
  onClose: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
}

export function SearchPageModal({ 
  isOpen, 
  onClose,
  searchQuery,
  onSearchChange
}: SearchPageModalProps) {
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
    <Portal>
      {/* Full screen backdrop to capture all clicks */}
      <div 
        className="fixed inset-0 z-[2000] bg-black/20" 
        onClick={onClose}
      />
      <div id="search-page-modal-container" className="fixed top-0 left-0 right-0 z-[2001] pt-2">
        <div id="search-page-modal-inner" className="container mx-auto px-4">
          <div id="search-page-modal-flex" className="flex items-center h-16">
          {/* Spacer for logo area */}
          <div className="flex-1" />
          
          {/* Center - Search (same positioning as NavBar) */}
          <div id="search-page-modal-search-wrapper" className="flex-1 max-w-xl mx-8 relative z-[2002]">
            <SearchBar
              mode="page-search"
              placeholder="Search for tokens..."
              searchQuery={searchQuery}
              onSearchChange={onSearchChange}
              onClose={onClose}
              autoFocus={true}
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