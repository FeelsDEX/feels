import {
  BaseMessageSignerWalletAdapter,
  WalletName,
  WalletNotConnectedError,
  WalletNotReadyError,
  WalletPublicKeyError,
  WalletReadyState,
} from '@solana/wallet-adapter-base';
import { PublicKey, Transaction, VersionedTransaction } from '@solana/web3.js';

interface OKXWindow {
  okxwallet?: {
    solana?: {
      isOkxWallet?: boolean;
      connect(): Promise<{ publicKey: PublicKey }>;
      disconnect(): Promise<void>;
      signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T>;
      signAllTransactions<T extends Transaction | VersionedTransaction>(transactions: T[]): Promise<T[]>;
      signMessage(message: Uint8Array): Promise<{ signature: Uint8Array }>;
      publicKey?: PublicKey;
      isConnected?: boolean;
    };
  };
}

declare const window: OKXWindow;

export interface OKXWalletAdapterConfig {}

export const OKXWalletName = 'OKX Wallet' as WalletName<'OKX Wallet'>;

export class OKXWalletAdapter extends BaseMessageSignerWalletAdapter {
  name = OKXWalletName;
  url = 'https://www.okx.com/web3';
  icon = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNDAiIGhlaWdodD0iNDAiIHZpZXdCb3g9IjAgMCA0MCA0MCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3Qgd2lkdGg9IjQwIiBoZWlnaHQ9IjQwIiByeD0iOCIgZmlsbD0iYmxhY2siLz4KPHBhdGggZD0iTTI4LjA0ODggMTJIMjMuMzY1NkwxNi4xOTc2IDE5LjE2OEwxOS43ODE2IDIyLjc1Mkw5LjA0ODgzIDMzLjQ4NDhWMjguODAxNkwxOS43ODE2IDI4LjgwMTZMMjguMDQ4OCAyMC41MzQ0VjEyWiIgZmlsbD0id2hpdGUiLz4KPHBhdGggZD0iTTExLjk1MTIgMjhMMTYuNjM0NCAyOEwyMy44MDI0IDIwLjgzMkwyMC4yMTg0IDE3LjI0OEwzMC45NTEyIDYuNTE1MlYxMS4xOTg0TDIwLjIxODQgMTEuMTk4NEwxMS45NTEyIDE5LjQ2NTZWMjhaIiBmaWxsPSJ3aGl0ZSIvPgo8L3N2Zz4K';
  supportedTransactionVersions = new Set(['legacy'] as const);

  private _connecting: boolean;
  private _publicKey: PublicKey | null;
  private _readyState: WalletReadyState =
    typeof window === 'undefined' || typeof document === 'undefined'
      ? WalletReadyState.Unsupported
      : WalletReadyState.Loadable;

  constructor(_config: OKXWalletAdapterConfig = {}) {
    super();
    this._connecting = false;
    this._publicKey = null;

    if (this._readyState !== WalletReadyState.Unsupported) {
      if (typeof window !== 'undefined') {
        if (window.okxwallet?.solana?.isOkxWallet) {
          this._readyState = WalletReadyState.Installed;
        } else {
          // Check periodically if OKX is injected
          const checkOKX = () => {
            if (window.okxwallet?.solana?.isOkxWallet) {
              this._readyState = WalletReadyState.Installed;
              this.emit('readyStateChange', this._readyState);
            }
          };
          
          // Check immediately
          checkOKX();
          
          // Check again after short delays (extension might still be loading)
          setTimeout(checkOKX, 100);
          setTimeout(checkOKX, 500);
          setTimeout(checkOKX, 1000);
        }
      }
    }
  }

  get publicKey() {
    return this._publicKey;
  }

  get connecting() {
    return this._connecting;
  }

  get readyState() {
    return this._readyState;
  }

  async connect(): Promise<void> {
    try {
      if (this.connected || this.connecting) return;
      if (this._readyState !== WalletReadyState.Installed) throw new WalletNotReadyError();

      this._connecting = true;

      const wallet = window.okxwallet?.solana;
      if (!wallet) throw new WalletNotReadyError();

      let publicKey: PublicKey;
      try {
        const response = await wallet.connect();
        publicKey = response.publicKey;
      } catch (error: any) {
        throw new WalletNotConnectedError(error?.message, error);
      }

      if (!publicKey) throw new WalletPublicKeyError();

      this._publicKey = publicKey;
      this.emit('connect', publicKey);
    } catch (error: any) {
      this.emit('error', error);
      throw error;
    } finally {
      this._connecting = false;
    }
  }

  async disconnect(): Promise<void> {
    const wallet = window.okxwallet?.solana;

    if (wallet) {
      await wallet.disconnect();
    }

    this._publicKey = null;
    this.emit('disconnect');
  }

  async signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T> {
    try {
      const wallet = window.okxwallet?.solana;
      if (!wallet) throw new WalletNotConnectedError();

      try {
        return await wallet.signTransaction(transaction);
      } catch (error: any) {
        throw error;
      }
    } catch (error: any) {
      this.emit('error', error);
      throw error;
    }
  }

  override async signAllTransactions<T extends Transaction | VersionedTransaction>(
    transactions: T[]
  ): Promise<T[]> {
    try {
      const wallet = window.okxwallet?.solana;
      if (!wallet) throw new WalletNotConnectedError();

      try {
        return await wallet.signAllTransactions(transactions);
      } catch (error: any) {
        throw error;
      }
    } catch (error: any) {
      this.emit('error', error);
      throw error;
    }
  }

  async signMessage(message: Uint8Array): Promise<Uint8Array> {
    try {
      const wallet = window.okxwallet?.solana;
      if (!wallet) throw new WalletNotConnectedError();

      try {
        const { signature } = await wallet.signMessage(message);
        return signature;
      } catch (error: any) {
        throw error;
      }
    } catch (error: any) {
      this.emit('error', error);
      throw error;
    }
  }
}