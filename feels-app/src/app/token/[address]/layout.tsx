export default function TokenLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}

// Disable static generation for dynamic token routes
export const dynamic = 'force-dynamic';
export const fetchCache = 'force-no-store';