'use client';

import { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { CheckCircle, Copy, Terminal, Database, Rocket, AlertCircle } from 'lucide-react';

export function SetupInstructions() {
  const [copiedStep, setCopiedStep] = useState<string | null>(null);

  const copyToClipboard = (text: string, step: string) => {
    navigator.clipboard.writeText(text);
    setCopiedStep(step);
    setTimeout(() => setCopiedStep(null), 2000);
  };

  const QuickStartSteps = [
    {
      id: 'validator',
      title: 'Start Validator & Deploy',
      command: './start-geyser-devnet.sh',
      description: 'Starts local Solana validator with Geyser plugin and deploys the Feels program',
    },
    {
      id: 'indexer',
      title: 'Start Indexer',
      command: './start-indexer.sh',
      description: 'Starts the indexer to capture on-chain events and serve API',
    },
    {
      id: 'app',
      title: 'Start App',
      command: 'cd feels-app && npm run dev',
      description: 'Start the Next.js application',
    },
  ];

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Rocket className="h-5 w-5" />
          Local Development Setup
        </CardTitle>
        <CardDescription>
          Get your local Feels Protocol environment running
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="quickstart" className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="quickstart">Quick Start</TabsTrigger>
            <TabsTrigger value="detailed">Detailed Setup</TabsTrigger>
            <TabsTrigger value="troubleshooting">Troubleshooting</TabsTrigger>
          </TabsList>

          <TabsContent value="quickstart" className="space-y-4">
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                Make sure you are in the project root directory before running these commands
              </AlertDescription>
            </Alert>

            <div className="space-y-4">
              {QuickStartSteps.map((step, index) => (
                <div key={step.id} className="border rounded-lg p-4">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <h3 className="font-medium mb-1">
                        Step {index + 1}: {step.title}
                      </h3>
                      <p className="text-sm text-muted-foreground mb-2">
                        {step.description}
                      </p>
                      <div className="flex items-center gap-2">
                        <code className="flex-1 bg-muted px-3 py-2 rounded text-sm font-mono">
                          {step.command}
                        </code>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => copyToClipboard(step.command, step.id)}
                        >
                          {copiedStep === step.id ? (
                            <CheckCircle className="h-4 w-4 text-primary" />
                          ) : (
                            <Copy className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <Alert className="bg-primary/10 border-primary/20">
              <CheckCircle className="h-4 w-4 text-primary" />
              <AlertDescription className="text-primary-foreground">
                Once all services are running, you can access the app at{' '}
                <a href="http://localhost:3000" className="font-medium underline">
                  http://localhost:3000
                </a>
              </AlertDescription>
            </Alert>
          </TabsContent>

          <TabsContent value="detailed" className="space-y-4">
            <div className="space-y-4">
              <div>
                <h3 className="font-medium mb-2 flex items-center gap-2">
                  <Terminal className="h-4 w-4" />
                  Prerequisites
                </h3>
                <ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground ml-6">
                  <li>Solana CLI tools (v1.17+)</li>
                  <li>Anchor framework (v0.31+)</li>
                  <li>Node.js (v18+) and npm</li>
                  <li>Rust toolchain</li>
                  <li>Nix package manager (optional but recommended)</li>
                </ul>
              </div>

              <div>
                <h3 className="font-medium mb-2">1. Build the Protocol</h3>
                <div className="space-y-2">
                  <code className="block bg-muted px-3 py-2 rounded text-sm font-mono">
                    cd programs/feels && cargo build-sbf
                  </code>
                  <p className="text-sm text-muted-foreground">
                    Compiles the Feels Protocol program for Solana BPF
                  </p>
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-2">2. Start Services</h3>
                <div className="space-y-3">
                  <div>
                    <p className="text-sm font-medium mb-1">Validator with Geyser:</p>
                    <code className="block bg-muted px-3 py-2 rounded text-sm font-mono">
                      ./start-geyser-devnet.sh
                    </code>
                    <p className="text-sm text-muted-foreground mt-1">
                      - RPC: http://localhost:8899<br />
                      - WebSocket: ws://localhost:8900<br />
                      - Geyser gRPC: http://localhost:10000
                    </p>
                  </div>

                  <div>
                    <p className="text-sm font-medium mb-1">Indexer:</p>
                    <code className="block bg-muted px-3 py-2 rounded text-sm font-mono">
                      ./start-indexer.sh
                    </code>
                    <p className="text-sm text-muted-foreground mt-1">
                      API endpoint: http://localhost:8080
                    </p>
                  </div>
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-2">3. Configure the App</h3>
                <div className="space-y-2">
                  <p className="text-sm text-muted-foreground">
                    Copy the example environment file:
                  </p>
                  <code className="block bg-muted px-3 py-2 rounded text-sm font-mono">
                    cd feels-app && cp .env.local.example .env.local
                  </code>
                  <p className="text-sm text-muted-foreground">
                    The default configuration should work for local development.
                  </p>
                </div>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="troubleshooting" className="space-y-4">
            <div className="space-y-4">
              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2">Port Conflicts</h4>
                <p className="text-sm text-muted-foreground mb-2">
                  If you see &ldquo;address already in use&rdquo; errors:
                </p>
                <code className="block bg-muted px-3 py-2 rounded text-sm font-mono">
                  # Kill processes on common ports<br />
                  pkill -f solana-test-validator<br />
                  pkill -f feels-indexer<br />
                  lsof -ti:8899 | xargs kill -9  # RPC<br />
                  lsof -ti:8080 | xargs kill -9  # Indexer API<br />
                  lsof -ti:10000 | xargs kill -9 # Geyser
                </code>
              </div>

              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2">Indexer Not Connecting</h4>
                <p className="text-sm text-muted-foreground mb-2">
                  If the indexer can&apos;t connect to Geyser:
                </p>
                <ul className="list-disc list-inside text-sm text-muted-foreground ml-4 space-y-1">
                  <li>Ensure the validator started successfully</li>
                  <li>Check Geyser is listening: <code className="font-mono text-xs">nc -z localhost 10000</code></li>
                  <li>The indexer will fall back to RPC polling if Geyser is unavailable</li>
                </ul>
              </div>

              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2">Program Deployment Failed</h4>
                <p className="text-sm text-muted-foreground mb-2">
                  If the program fails to deploy:
                </p>
                <ul className="list-disc list-inside text-sm text-muted-foreground ml-4 space-y-1">
                  <li>Ensure you have enough SOL: <code className="font-mono text-xs">solana airdrop 10</code></li>
                  <li>Check the program built: <code className="font-mono text-xs">ls programs/feels/target/deploy/feels.so</code></li>
                  <li>Try manual deployment: <code className="font-mono text-xs">solana program deploy programs/feels/target/deploy/feels.so</code></li>
                </ul>
              </div>

              <div className="border rounded-lg p-4">
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <Database className="h-4 w-4" />
                  Optional Services
                </h4>
                <p className="text-sm text-muted-foreground">
                  The indexer works with just RocksDB by default. PostgreSQL, Redis, and Tantivy are optional:
                </p>
                <ul className="list-disc list-inside text-sm text-muted-foreground ml-4 space-y-1 mt-2">
                  <li>PostgreSQL: For complex queries and relational data</li>
                  <li>Redis: For caching and real-time updates</li>
                  <li>Tantivy: For full-text search capabilities</li>
                </ul>
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}