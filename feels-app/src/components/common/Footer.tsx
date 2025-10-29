'use client';

import Link from 'next/link';
import { useDeveloperMode } from '@/contexts/DeveloperModeContext';

export function Footer() {
  const { isDeveloperMode } = useDeveloperMode();

  return (
    <footer className="py-6 md:py-10 mt-auto">
      <div className="container mx-auto px-4 md:px-6">
        <div className="relative flex flex-col md:flex-row items-center md:items-center gap-4 md:gap-0">
          <div className="flex-1 flex justify-center md:justify-start">
            <div className="flex flex-row md:flex-col space-x-4 md:space-x-0 md:space-y-1 text-center md:text-left">
              <Link href="/docs" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                docs
              </Link>
              <Link href="/blog" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                blog
              </Link>
            </div>
          </div>
          <p className="text-center text-muted-foreground order-first md:order-none">
            feels good man
          </p>
          <div className="flex-1 flex justify-center md:justify-end">
            <div className="flex flex-row md:flex-col space-x-4 md:space-x-0 md:space-y-1 text-center md:text-right">
              {isDeveloperMode && (
                <>
                  <Link href="/info" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                    info
                  </Link>
                  <Link href="/control" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                    control
                  </Link>
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </footer>
  );
}

