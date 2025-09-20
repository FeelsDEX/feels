import {
  BaseMessageSignerWalletAdapter,
  WalletName,
  WalletNotConnectedError,
  WalletNotReadyError,
  WalletPublicKeyError,
  WalletReadyState,
} from '@solana/wallet-adapter-base';
import { PublicKey, Transaction, VersionedTransaction } from '@solana/web3.js';

interface MagicEdenWindow {
  magicEden?: {
    solana?: {
      isMagicEden?: boolean;
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

declare const window: MagicEdenWindow;

export interface MagicEdenWalletAdapterConfig {}

export const MagicEdenWalletName = 'Magic Eden' as WalletName<'Magic Eden'>;

export class MagicEdenWalletAdapter extends BaseMessageSignerWalletAdapter {
  name = MagicEdenWalletName;
  url = 'https://wallet.magiceden.io/';
  icon = 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgdmlld0JveD0iMCAwIDEwMCAxMDAiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxyZWN0IHdpZHRoPSIxMDAiIGhlaWdodD0iMTAwIiByeD0iMjAiIGZpbGw9IiNFNDJEQzEiLz4KPHBhdGggZD0iTTcwLjc1IDI5LjI1SDI5LjI1QzI3LjE3ODkgMjkuMjUgMjUuNSAzMC45Mjg5IDI1LjUgMzNWNjdDMjUuNSA2OS4wNzExIDI3LjE3ODkgNzAuNzUgMjkuMjUgNzAuNzVINzAuNzVDNzIuODIxMSA3MC43NSA3NC41IDY5LjA3MTEgNzQuNSA2N1YzM0M3NC41IDMwLjkyODkgNzIuODIxMSAyOS4yNSA3MC43NSAyOS4yNVoiIGZpbGw9IndoaXRlIi8+CjxwYXRoIGQ9Ik02MC41IDUyLjI1SDM5LjVDMzcuNDI4OSA1Mi4yNSAzNS43NSA1My45Mjg5IDM1Ljc1IDU2VjYxQzM1Ljc1IDYzLjA3MTEgMzcuNDI4OSA2NC43NSAzOS41IDY0Ljc1SDYwLjVDNjIuNTcxMSA2NC43NSA2NC4yNSA2My4wNzExIDY0LjI1IDYxVjU2QzY0LjI1IDUzLjkyODkgNjIuNTcxMSA1Mi4yNSA2MC41IDUyLjI1WiIgZmlsbD0iI0U0MkRDMSIvPgo8cGF0aCBkPSJNNDUgMzkuMjVDNDUgNDEuMzIxMSA0My4zMjExIDQzIDQxLjI1IDQzQzM5LjE3ODkgNDMgMzcuNSA0MS4zMjExIDM3LjUgMzkuMjVDMzcuNSAzNy4xNzg5IDM5LjE3ODkgMzUuNSA0MS4yNSAzNS41QzQzLjMyMTEgMzUuNSA0NSAzNy4xNzg5IDQ1IDM5LjI1WiIgZmlsbD0iI0U0MkRDMSIvPgo8cGF0aCBkPSJNNjIuNSAzOS4yNUM2Mi41IDQxLjMyMTEgNjAuODIxMSA0MyA1OC43NSA0M0M1Ni42Nzg5IDQzIDU1IDQxLjMyMTEgNTUgMzkuMjVDNTUgMzcuMTc4OSA1Ni42Nzg5IDM1LjUgNTguNzUgMzUuNUM2MC44MjExIDM1LjUgNjIuNSAzNy4xNzg5IDYyLjUgMzkuMjVaIiBmaWxsPSIjRTQyREMxIi8+Cjwvc3ZnPgo=';
  supportedTransactionVersions = new Set(['legacy'] as const);

  private _connecting: boolean;
  private _publicKey: PublicKey | null;
  private _readyState: WalletReadyState =
    typeof window === 'undefined' || typeof document === 'undefined'
      ? WalletReadyState.Unsupported
      : WalletReadyState.Loadable;

  constructor(config: MagicEdenWalletAdapterConfig = {}) {
    super();
    this._connecting = false;
    this._publicKey = null;

    if (this._readyState !== WalletReadyState.Unsupported) {
      if (typeof window !== 'undefined') {
        if (window.magicEden?.solana?.isMagicEden) {
          this._readyState = WalletReadyState.Installed;
        } else {
          // Check periodically if Magic Eden is injected
          const checkMagicEden = () => {
            if (window.magicEden?.solana?.isMagicEden) {
              this._readyState = WalletReadyState.Installed;
              this.emit('readyStateChange', this._readyState);
            }
          };
          
          // Check immediately
          checkMagicEden();
          
          // Check again after short delays (extension might still be loading)
          setTimeout(checkMagicEden, 100);
          setTimeout(checkMagicEden, 500);
          setTimeout(checkMagicEden, 1000);
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

      const wallet = window.magicEden?.solana;
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
    const wallet = window.magicEden?.solana;

    if (wallet) {
      await wallet.disconnect();
    }

    this._publicKey = null;
    this.emit('disconnect');
  }

  async signTransaction<T extends Transaction | VersionedTransaction>(transaction: T): Promise<T> {
    try {
      const wallet = window.magicEden?.solana;
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
      const wallet = window.magicEden?.solana;
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
      const wallet = window.magicEden?.solana;
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