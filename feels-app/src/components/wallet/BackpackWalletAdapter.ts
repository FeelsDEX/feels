import {
  BaseMessageSignerWalletAdapter,
  WalletName,
  WalletNotConnectedError,
  WalletNotReadyError,
  WalletPublicKeyError,
  WalletReadyState,
  SupportedTransactionVersions
} from '@solana/wallet-adapter-base';
import { PublicKey, Transaction, VersionedTransaction, TransactionVersion } from '@solana/web3.js';

interface BackpackWindow {
  backpack?: {
    solana?: {
      isBackpack?: boolean;
      connect(): Promise<{ publicKey: PublicKey }>;
      disconnect(): Promise<void>;
      signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T>;
      signAllTransactions<T extends Transaction | VersionedTransaction>(transactions: T[]): Promise<T[]>;
      signMessage(message: Uint8Array): Promise<{ signature: Uint8Array }>;
    };
  };
}

declare const window: BackpackWindow;

export interface BackpackWalletAdapterConfig {}

export const BackpackWalletName = 'Backpack' as WalletName<'Backpack'>;

export class BackpackWalletAdapter extends BaseMessageSignerWalletAdapter {
  name = BackpackWalletName;
  url = 'https://www.backpack.app/';
  icon = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgdmlld0JveD0iMCAwIDEwMCAxMDAiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxyZWN0IHdpZHRoPSIxMDAiIGhlaWdodD0iMTAwIiByeD0iMjAiIGZpbGw9IiNFMzNFM0YiLz4KPHBhdGggZD0iTTUwIDI1VjI1QzU5LjY2NSAyNSA2Ny41IDMyLjgzNSA2Ny41IDQyLjVWNTcuNUM2Ny41IDY3LjE2NSA1OS42NjUgNzUgNTAgNzVWNzVDNDAuMzM1IDc1IDMyLjUgNjcuMTY1IDMyLjUgNTcuNVY0Mi41QzMyLjUgMzIuODM1IDQwLjMzNSAyNSA1MCAyNVoiIHN0cm9rZT0iI0ZGRkZGRiIgc3Ryb2tlLXdpZHRoPSI1Ii8+Cjwvc3ZnPg==';
  readonly supportedTransactionVersions: SupportedTransactionVersions = new Set<TransactionVersion>(['legacy', 0]);

  private _connecting: boolean;
  private _publicKey: PublicKey | null;
  private _readyState: WalletReadyState =
    typeof window === 'undefined' || typeof document === 'undefined'
      ? WalletReadyState.Unsupported
      : WalletReadyState.Loadable;

  constructor(config: BackpackWalletAdapterConfig = {}) {
    super();
    this._connecting = false;
    this._publicKey = null;

    if (this._readyState !== WalletReadyState.Unsupported) {
      if (typeof window !== 'undefined') {
        if (window.backpack?.solana?.isBackpack) {
          this._readyState = WalletReadyState.Installed;
        } else {
          // Check periodically if Backpack is injected
          const checkBackpack = () => {
            if (window.backpack?.solana?.isBackpack) {
              this._readyState = WalletReadyState.Installed;
              this.emit('readyStateChange', this._readyState);
            }
          };
          
          // Check immediately
          checkBackpack();
          
          // Check again after a short delay (extension might still be loading)
          setTimeout(checkBackpack, 100);
          setTimeout(checkBackpack, 500);
          setTimeout(checkBackpack, 1000);
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

      const wallet = window.backpack?.solana;
      if (!wallet) throw new WalletNotReadyError();

      let account: { publicKey: PublicKey };
      try {
        account = await wallet.connect();
      } catch (error: any) {
        throw new WalletNotConnectedError(error?.message, error);
      }

      if (!account.publicKey) throw new WalletPublicKeyError();

      this._publicKey = account.publicKey;
      this.emit('connect', account.publicKey);
    } catch (error: any) {
      this.emit('error', error);
      throw error;
    } finally {
      this._connecting = false;
    }
  }

  async disconnect(): Promise<void> {
    const wallet = window.backpack?.solana;

    if (wallet) {
      await wallet.disconnect();
    }

    this._publicKey = null;
    this.emit('disconnect');
  }

  async signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T> {
    try {
      const wallet = window.backpack?.solana;
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

  async signAllTransactions<T extends Transaction | VersionedTransaction>(
    transactions: T[]
  ): Promise<T[]> {
    try {
      const wallet = window.backpack?.solana;
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
      const wallet = window.backpack?.solana;
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