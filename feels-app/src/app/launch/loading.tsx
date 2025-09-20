import { Card, CardContent } from '@/components/ui/card';
import { Loader2 } from 'lucide-react';

export default function LaunchLoading() {
  return (
    <div className="container mx-auto px-4 py-8">
      <div className="max-w-6xl mx-auto">
        <Card className="max-w-2xl mx-auto">
          <CardContent className="p-12">
            <div className="flex flex-col items-center justify-center space-y-4">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              <p className="text-sm text-muted-foreground">Loading launch page...</p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}