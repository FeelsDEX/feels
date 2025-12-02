// Sidebar component with tabs for Filters and Troll Box
'use client';

import { useState, useRef, useEffect } from 'react';
import { TokenSearchFiltersContent } from './TokenSearchFiltersContent';
import { SelectedFacets } from '@/utils/token-search';
import { Input } from '@/components/ui/input';

interface SidebarTabsProps {
  selectedFacets: SelectedFacets;
  toggleFacet: (category: keyof SelectedFacets, value: string) => void;
  clearFilters: () => void;
  facetCounts: any;
  isChartView?: boolean;
}

type TabType = 'trollbox' | 'fyi' | 'filters';

interface ChatMessage {
  id: number;
  user: string;
  message: string;
  timestamp: string;
}

// Wojak-themed mock chat messages
const INITIAL_MESSAGES: ChatMessage[] = [
  { id: 1, user: 'anon_wojak', message: 'wen moon ser?', timestamp: '2m ago' },
  { id: 2, user: 'pepe_hands', message: 'NGMI if you sell now', timestamp: '5m ago' },
  { id: 3, user: 'diamond_wojak', message: 'just bought the dip', timestamp: '8m ago' },
  { id: 4, user: 'cope_lord', message: 'feels good man', timestamp: '12m ago' },
  { id: 5, user: 'anon_wojak', message: 'why do I always buy at ATH', timestamp: '15m ago' },
  { id: 6, user: 'hopium_dealer', message: 'trust the process', timestamp: '18m ago' },
  { id: 7, user: 'rekt_wojak', message: 'I am financially ruined', timestamp: '22m ago' },
  { id: 8, user: 'gigachad_trader', message: 'stayed calm and made 10x', timestamp: '25m ago' },
  { id: 9, user: 'degen_ape', message: 'YOLO everything into this', timestamp: '30m ago' },
  { id: 10, user: 'anon_wojak', message: 'should have listened to my wife', timestamp: '35m ago' },
];

export function SidebarTabs({ selectedFacets, toggleFacet, clearFilters, facetCounts, isChartView = false }: SidebarTabsProps) {
  const [activeTab, setActiveTab] = useState<TabType>('trollbox');
  const [messages, setMessages] = useState<ChatMessage[]>(INITIAL_MESSAGES);
  const [inputValue, setInputValue] = useState('');
  const chatContainerRef = useRef<HTMLDivElement>(null);
  
  // Force trollbox tab when in chart view
  useEffect(() => {
    if (isChartView && activeTab === 'filters') {
      setActiveTab('trollbox');
    }
  }, [isChartView, activeTab]);
  
  // Auto-scroll to bottom when new messages are added
  useEffect(() => {
    if (activeTab === 'trollbox' && chatContainerRef.current) {
      chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight;
    }
  }, [messages, activeTab]);
  
  // Handle sending a new message
  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!inputValue.trim()) return;
    
    const newMessage: ChatMessage = {
      id: Date.now(),
      user: 'anon', // Default user name for all messages
      message: inputValue.trim(),
      timestamp: 'just now'
    };
    
    setMessages(prev => [...prev, newMessage]);
    setInputValue('');
  };

  return (
    <div className={`bg-background border border-border rounded-lg overflow-hidden ${
      activeTab === 'trollbox' ? 'h-[calc(100vh-8.5rem)] flex flex-col' : ''
    }`}>
      {/* Tab Navigation */}
      <div className="flex items-center justify-between border-b border-border flex-shrink-0">
        <div className="flex w-full">
          <button
            onClick={() => setActiveTab('trollbox')}
            className={`flex-1 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'trollbox'
                ? 'text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Troll Box
            {activeTab === 'trollbox' && (
              <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-16 h-0.5 bg-primary" />
            )}
          </button>
          <button
            onClick={() => !isChartView && setActiveTab('filters')}
            disabled={isChartView}
            className={`flex-1 py-4 text-sm font-medium transition-colors relative ${
              isChartView
                ? 'text-muted-foreground/50 cursor-not-allowed'
                : activeTab === 'filters'
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Filters
            {activeTab === 'filters' && !isChartView && (
              <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-16 h-0.5 bg-primary" />
            )}
          </button>
          <button
            onClick={() => setActiveTab('fyi')}
            className={`flex-1 py-4 text-sm font-medium transition-colors relative ${
              activeTab === 'fyi'
                ? 'text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            FYI
            {activeTab === 'fyi' && (
              <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-16 h-0.5 bg-primary" />
            )}
          </button>
        </div>
      </div>

      {/* Tab Content */}
      {activeTab === 'trollbox' ? (
        <div className="flex-1 flex flex-col px-4 pb-4 min-h-0">
          {/* Chat messages with fade gradient */}
          <div className="flex-1 relative min-h-0">
            {/* Fade gradient overlay at top */}
            <div className="absolute top-0 left-0 right-0 h-8 bg-gradient-to-b from-background to-transparent z-10 pointer-events-none" />
            
            {/* Fade gradient overlay at bottom */}
            <div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-background to-transparent z-10 pointer-events-none" />
            
            <div ref={chatContainerRef} className="absolute inset-0 space-y-3 overflow-y-auto pr-2">
              {messages.map((msg) => (
                <div key={msg.id} className="space-y-1">
                  <div className="flex items-baseline gap-2">
                    <span className="text-xs font-medium text-primary">
                      {msg.user}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {msg.timestamp}
                    </span>
                  </div>
                  <p className="text-sm text-foreground">
                    {msg.message}
                  </p>
                </div>
              ))}
            </div>
          </div>

          {/* Chat input */}
          <form onSubmit={handleSendMessage} className="pt-5 border-t border-border flex-shrink-0">
            <div className="flex gap-2">
              <Input
                type="text"
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                placeholder="Type a message..."
                className="flex-1"
              />
              <button
                type="submit"
                disabled={!inputValue.trim()}
                className="px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                Send
              </button>
            </div>
          </form>
        </div>
      ) : activeTab === 'fyi' ? (
        <div className="px-4 pb-4 pt-4 space-y-4 overflow-y-auto text-sm">
          <div className="space-y-3">
            <h3 className="font-semibold text-primary">What Makes Feels Special?</h3>
            <p className="text-muted-foreground leading-relaxed">
              Feels converts short-term trading volatility into long-term value through a rising price floor.
            </p>
          </div>

          <div className="space-y-3">
            <h3 className="font-semibold text-primary">How the Floor Price Works</h3>
            <p className="text-muted-foreground leading-relaxed">
              Each token has a guaranteed price floor calculated as:
            </p>
            <div className="bg-muted/30 p-3 rounded font-mono text-xs">
              Floor Price = Pool Reserves / Circulating Supply
            </div>
            <p className="text-muted-foreground leading-relaxed">
              This ensures the protocol can buy back every circulating token at the floor price. 
              The floor is monotonic (only rises, never falls) and increases as the protocol accumulates 
              value from trading fees, liquid staking yield, and (soon) fees from leverage. Protocol-owned liquidity maintains a hard buy 
              wall at this floor, providing a permanent exit guarantee for holders.
            </p>
          </div>
        </div>
      ) : (
        <div className="px-4 pb-4 pt-4">
          <TokenSearchFiltersContent
            selectedFacets={selectedFacets}
            toggleFacet={toggleFacet}
            clearFilters={clearFilters}
            facetCounts={facetCounts}
          />
        </div>
      )}
    </div>
  );
}

