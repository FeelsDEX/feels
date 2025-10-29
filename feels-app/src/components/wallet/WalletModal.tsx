'use client';

import { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletReadyState } from '@solana/wallet-adapter-base';
import Image from 'next/image';
import { Plus, Minus, CheckCircle2, LogOut } from 'lucide-react';
import wojakImage from '@/assets/images/wojak.png';

interface WalletModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function WalletModal({ open, onOpenChange }: WalletModalProps) {
  const { wallets, select, wallet, connect, connecting, disconnect, connected, publicKey } = useWallet();
  const [selectedWallet, setSelectedWallet] = useState<string | null>(null);
  const [showMoreWallets, setShowMoreWallets] = useState(false);
  const [hoveredWallet, setHoveredWallet] = useState<string | null>(null);

  // First deduplicate wallets by name
  const deduplicatedWallets = wallets.filter((wallet, index, self) => 
    index === self.findIndex((w) => w.adapter.name === wallet.adapter.name)
  );

  // Group wallets by ready state
  const { installed, loadable, notDetected } = deduplicatedWallets.reduce(
    (acc, wallet) => {
      if (wallet.readyState === WalletReadyState.Installed) {
        acc.installed.push(wallet);
      } else if (wallet.readyState === WalletReadyState.Loadable) {
        acc.loadable.push(wallet);
      } else {
        acc.notDetected.push(wallet);
      }
      return acc;
    },
    {
      installed: [] as typeof wallets,
      loadable: [] as typeof wallets,
      notDetected: [] as typeof wallets,
    }
  );


  const handleWalletSelect = async (walletName: string) => {
    try {
      // If clicking on the currently connected wallet, disconnect it
      if (connected && wallet?.adapter.name === walletName) {
        await disconnect();
        onOpenChange(false);
        return;
      }
      
      // If a different wallet is already connected, disconnect it first
      if (connected && wallet?.adapter.name !== walletName) {
        await disconnect();
      }
      
      setSelectedWallet(walletName);
      select(walletName as any);
      // Don't close the modal immediately - wait for connection in useEffect
    } catch (error) {
      console.error('Failed to select wallet:', error);
      setSelectedWallet(null);
    }
  };

  useEffect(() => {
    if (wallet && !(wallet.adapter as any).connected && selectedWallet === wallet.adapter.name) {
      connect()
        .then(() => {
          // Close modal after successful connection
          onOpenChange(false);
          setSelectedWallet(null);
        })
        .catch((error) => {
          console.error('Failed to connect wallet:', error);
          setSelectedWallet(null);
        });
    }
  }, [wallet, connect, selectedWallet, onOpenChange]);

  // Reset the expanded state when modal closes
  useEffect(() => {
    if (!open) {
      setShowMoreWallets(false);
    }
    return undefined;
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Connect a wallet</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {installed.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-muted-foreground mb-2">
                Installed wallets
              </h3>
              <div className="space-y-2">
                {installed.map((installedWallet) => {
                  const isConnected = connected && wallet?.adapter.name === installedWallet.adapter.name;
                  const isConnecting = connecting && selectedWallet === installedWallet.adapter.name;
                  const isHovered = hoveredWallet === installedWallet.adapter.name;
                  
                  return isConnected ? (
                    <div key={installedWallet.adapter.name} className="flex w-full gap-2">
                      <Button
                        variant="outline"
                        className="flex-1 justify-start text-foreground hover:bg-primary hover:text-white hover:border-primary focus-visible:ring-0 focus-visible:ring-offset-0 transition-colors"
                        onClick={() => handleWalletSelect(installedWallet.adapter.name)}
                        disabled={isConnecting}
                        onMouseEnter={() => setHoveredWallet(installedWallet.adapter.name)}
                        onMouseLeave={() => setHoveredWallet(null)}
                      >
                        <div className="flex items-center gap-3">
                          {installedWallet.adapter.icon && (
                            <Image
                              src={installedWallet.adapter.icon}
                              alt={installedWallet.adapter.name}
                              width={24}
                              height={24}
                              className={`rounded-md ${
                                installedWallet.adapter.name === 'Ledger' ? 'invert' : ''
                              }`}
                              style={{ width: '24px', height: '24px', objectFit: 'contain' }}
                            />
                          )}
                          <span>{installedWallet.adapter.name}</span>
                        </div>
                        <div className="ml-auto">
                          {isHovered ? (
                            <LogOut className="h-4 w-4" />
                          ) : (
                            <CheckCircle2 className="h-4 w-4 text-primary" />
                          )}
                        </div>
                      </Button>
                      {publicKey && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onOpenChange(false);
                            window.location.href = `/account/${publicKey.toBase58()}`;
                          }}
                          className="flex flex-1 items-center justify-start h-10 px-4 text-sm font-medium border border-input bg-background text-foreground rounded-md hover:bg-primary hover:text-white hover:border-primary transition-colors focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
                          title="View Profile"
                        >
                          <div className="flex items-center gap-3">
                            <div className="rounded-md bg-white p-0.5">
                              <Image
                                src={wojakImage}
                                alt="Profile"
                                width={20}
                                height={20}
                                className="rounded-md"
                                style={{ width: '20px', height: '20px', objectFit: 'contain' }}
                              />
                            </div>
                            <span>Your Profile</span>
                          </div>
                        </button>
                      )}
                    </div>
                  ) : (
                    <Button
                      key={installedWallet.adapter.name}
                      variant="outline"
                      className="w-full justify-start"
                      onClick={() => handleWalletSelect(installedWallet.adapter.name)}
                      disabled={isConnecting}
                    >
                      <div className="flex items-center gap-3">
                        {installedWallet.adapter.icon && (
                          <Image
                            src={installedWallet.adapter.icon}
                            alt={installedWallet.adapter.name}
                            width={24}
                            height={24}
                            className={`rounded-md ${
                              installedWallet.adapter.name === 'Ledger' ? 'invert' : ''
                            }`}
                            style={{ width: '24px', height: '24px', objectFit: 'contain' }}
                          />
                        )}
                        <span>{installedWallet.adapter.name}</span>
                      </div>
                      {isConnecting && (
                        <span className="ml-auto text-sm text-muted-foreground">
                          Connecting...
                        </span>
                      )}
                    </Button>
                  );
                })}
              </div>
            </div>
          )}

          {(loadable.length > 0 || notDetected.length > 0) && (
            <div>
              <Button
                variant="ghost"
                className="w-full justify-between p-0 hover:bg-transparent"
                onClick={() => setShowMoreWallets(!showMoreWallets)}
              >
                <h3 className="text-sm font-medium text-muted-foreground">
                  More wallets
                </h3>
                {showMoreWallets ? (
                  <Minus className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <Plus className="h-4 w-4 text-muted-foreground" />
                )}
              </Button>
              
              {showMoreWallets && (
                <div className="space-y-2 mt-2 max-h-64 overflow-y-auto">
                  {loadable.map((wallet) => (
                    <Button
                      key={wallet.adapter.name}
                      variant="outline"
                      className="w-full justify-start"
                      onClick={() => {
                        if (wallet.adapter.url) {
                          window.open(wallet.adapter.url, '_blank');
                        }
                      }}
                    >
                      <div className="flex items-center gap-3">
                        {wallet.adapter.icon && (
                          <Image
                            src={wallet.adapter.icon}
                            alt={wallet.adapter.name}
                            width={24}
                            height={24}
                            className={`rounded-md ${
                              wallet.adapter.name === 'Ledger' ? 'invert' : ''
                            }`}
                            style={{ width: '24px', height: '24px', objectFit: 'contain' }}
                          />
                        )}
                        <span>{wallet.adapter.name}</span>
                      </div>
                      <span className="ml-auto text-sm text-muted-foreground">
                        Install
                      </span>
                    </Button>
                  ))}
                  {notDetected.map((wallet) => (
                        <Button
                          key={wallet.adapter.name}
                          variant="outline"
                          className="w-full justify-start"
                          onClick={() => {
                            if (wallet.adapter.url) {
                              window.open(wallet.adapter.url, '_blank');
                            }
                          }}
                        >
                          <div className="flex items-center gap-3">
                            {wallet.adapter.icon && (
                              <Image
                                src={wallet.adapter.icon}
                                alt={wallet.adapter.name}
                                width={24}
                                height={24}
                                className="rounded-md"
                                style={{ width: '24px', height: '24px', objectFit: 'contain' }}
                              />
                            )}
                            <span>{wallet.adapter.name}</span>
                          </div>
                          <span className="ml-auto text-sm text-muted-foreground">
                            Install
                          </span>
                        </Button>
                  ))}
                </div>
              )}
            </div>
          )}

        </div>
      </DialogContent>
    </Dialog>
  );
}