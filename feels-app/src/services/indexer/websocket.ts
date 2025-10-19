// WebSocket client for real-time Feels Protocol updates

// Subscription types
export type SubscriptionType = 
  | { type: 'all_markets' }
  | { type: 'market'; address: string }
  | { type: 'swaps'; market?: string }
  | { type: 'positions'; user?: string }
  | { type: 'floor_updates'; market: string }
  | { type: 'price_updates'; market: string };

// Update event types
export type UpdateEvent = 
  | {
      type: 'market_update';
      market: string;
      sqrt_price: string;
      liquidity: string;
      current_tick: number;
      timestamp: number;
    }
  | {
      type: 'swap_event';
      market: string;
      user: string;
      amount_in: string;
      amount_out: string;
      token_in: string;
      token_out: string;
      price: number;
      timestamp: number;
    }
  | {
      type: 'position_update';
      position: string;
      market: string;
      owner: string;
      liquidity: string;
      tick_lower: number;
      tick_upper: number;
      timestamp: number;
    }
  | {
      type: 'floor_update';
      market: string;
      new_floor_tick: number;
      new_floor_price: number;
      timestamp: number;
    }
  | {
      type: 'price_update';
      market: string;
      price: number;
      price_change_24h: number;
      timestamp: number;
    }
  | {
      type: 'subscribed';
      id: string;
      subscriptions: SubscriptionType[];
    }
  | {
      type: 'error';
      code: string;
      message: string;
    };

export class FeelsWebSocketClient {
  private ws?: WebSocket;
  private url: string;
  private reconnectTimeout?: NodeJS.Timeout;
  private subscriptions: Map<string, SubscriptionType[]> = new Map();
  private eventHandlers: Map<string, (event: UpdateEvent) => void> = new Map();
  
  constructor(url: string) {
    this.url = url.replace('http:', 'ws:').replace('https:', 'wss:');
  }
  
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      // Only use WebSocket in browser environment
      if (typeof window === 'undefined') {
        reject(new Error('WebSocket is not available in server environment'));
        return;
      }
      
      try {
        this.ws = new WebSocket(`${this.url}/ws`);
        
        this.ws.onopen = () => {
          console.log('WebSocket connected');
          // Resubscribe to all previous subscriptions
          for (const [id, subs] of this.subscriptions) {
            this.ws?.send(JSON.stringify({
              type: 'subscribe',
              id,
              subscriptions: subs,
            }));
          }
          resolve();
        };
        
        this.ws.onmessage = (event) => {
          try {
            const update = JSON.parse(event.data) as UpdateEvent;
            // Dispatch to all registered handlers
            for (const handler of this.eventHandlers.values()) {
              handler(update);
            }
          } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
          }
        };
        
        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(error);
        };
        
        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          // Attempt reconnection after 5 seconds
          this.reconnectTimeout = setTimeout(() => {
            this.connect().catch(console.error);
          }, 5000);
        };
      } catch (error) {
        reject(error);
      }
    });
  }
  
  disconnect(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
  }
  
  subscribe(id: string, subscriptions: SubscriptionType[]): void {
    this.subscriptions.set(id, subscriptions);
    // Only check WebSocket state in browser environment
    if (typeof window !== 'undefined' && this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        id,
        subscriptions,
      }));
    }
  }
  
  unsubscribe(id: string): void {
    const subscriptions = this.subscriptions.get(id);
    // Only check WebSocket state in browser environment
    if (typeof window !== 'undefined' && subscriptions && this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'unsubscribe',
        id,
        subscriptions,
      }));
    }
    this.subscriptions.delete(id);
  }
  
  onUpdate(id: string, handler: (event: UpdateEvent) => void): void {
    this.eventHandlers.set(id, handler);
  }
  
  removeHandler(id: string): void {
    this.eventHandlers.delete(id);
  }
}

// Factory function for creating WebSocket client instances
export const createWebSocketClient = (baseUrl?: string): FeelsWebSocketClient => {
  const url = baseUrl || process.env['NEXT_PUBLIC_INDEXER_URL'] || 'http://localhost:8080';
  return new FeelsWebSocketClient(url);
};

