'use client';

import { useDataSource } from '@/contexts/DataSourceContext';
import { FallbackBanner } from '@/components/ui/fallback-banner';
import { useState, useEffect } from 'react';

export function GlobalNoMarketsBanner() {
  const { isUsingFallback } = useDataSource();
  const [isDismissed, setIsDismissed] = useState(false);

  // Reset dismissed state when fallback status changes
  useEffect(() => {
    if (!isUsingFallback) {
      setIsDismissed(false);
    }
  }, [isUsingFallback]);

  if (!isUsingFallback || isDismissed) {
    return null;
  }

  return (
    <div className="sticky top-0 z-50">
      <FallbackBanner
        variant="info"
        title="Using test data"
        message="No markets available yet. Create markets through the protocol to see real data."
        dismissible={true}
        onDismiss={() => setIsDismissed(true)}
        className="rounded-none border-x-0 border-t-0"
      />
    </div>
  );
}