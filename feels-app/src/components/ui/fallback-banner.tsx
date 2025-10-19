'use client';

import { useState } from 'react';
import { X } from 'lucide-react';

export type FallbackBannerVariant = 'warning' | 'info' | 'error';

interface FallbackBannerProps {
  variant?: FallbackBannerVariant;
  title?: string;
  message: string;
  dismissible?: boolean;
  onDismiss?: () => void;
  className?: string;
}

const variantStyles = {
  warning: {
    container: 'bg-amber-50 text-amber-800 border-amber-200',
    button: 'text-amber-800/80 hover:text-amber-900'
  },
  info: {
    container: 'bg-blue-50 text-blue-800 border-blue-200',
    button: 'text-blue-800/80 hover:text-blue-900'
  },
  error: {
    container: 'bg-red-50 text-red-800 border-red-200',
    button: 'text-red-800/80 hover:text-red-900'
  }
};

export function FallbackBanner({
  variant = 'info',
  title,
  message,
  dismissible = true,
  onDismiss,
  className = ''
}: FallbackBannerProps) {
  const [isVisible, setIsVisible] = useState(true);
  
  const handleDismiss = () => {
    setIsVisible(false);
    onDismiss?.();
  };

  if (!isVisible) {
    return null;
  }

  const styles = variantStyles[variant];

  return (
    <div className={`mb-4 relative p-3 rounded-md border ${styles.container} ${className}`}>
      <div className={dismissible ? 'pr-6' : ''}>
        {title && <strong>{title}:</strong>} {message}
      </div>
      {dismissible && (
        <button
          type="button"
          aria-label="Close"
          onClick={handleDismiss}
          className={`absolute right-4 top-[calc(50%-2px)] -translate-y-1/2 ${styles.button} hover:bg-black/5 rounded-full p-1 transition-colors`}
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}

// Specific banner variants for common use cases
export function TestDataFallbackBanner({ 
  className, 
  onDismiss 
}: { 
  className?: string; 
  onDismiss?: () => void; 
}) {
  return (
    <FallbackBanner
      variant="info"
      title="Using test data"
      message="No markets available yet. Create markets through the protocol to see real data."
      className={className}
      onDismiss={onDismiss}
    />
  );
}

export function ProgramFallbackBanner({ 
  className, 
  onDismiss 
}: { 
  className?: string; 
  onDismiss?: () => void; 
}) {
  return (
    <FallbackBanner
      variant="warning"
      message="Feels program not yet initialized. Falling back to test data."
      className={className}
      onDismiss={onDismiss}
    />
  );
}

export function IndexerErrorBanner({ 
  className, 
  onDismiss 
}: { 
  className?: string; 
  onDismiss?: () => void; 
}) {
  return (
    <FallbackBanner
      variant="error"
      message="Failed to load market data. Showing test data instead."
      className={className}
      onDismiss={onDismiss}
    />
  );
}