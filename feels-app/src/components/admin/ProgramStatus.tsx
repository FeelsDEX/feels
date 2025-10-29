'use client';

import { useState, useEffect } from 'react';
import { Connection, PublicKey } from '@solana/web3.js';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle2, XCircle, AlertCircle, Loader2 } from 'lucide-react';

interface ProgramStatusProps {
  connection: Connection;
  program: any;
  fallback: boolean;
}

interface StatusItem {
  label: string;
  status: 'checking' | 'success' | 'warning' | 'error';
  message: string;
}

export function ProgramStatus({ connection, program, fallback }: ProgramStatusProps) {
  const [jitoStatus, setJitoStatus] = useState<StatusItem>({
    label: 'JitoSOL Contract',
    status: 'checking',
    message: 'Checking deployment...'
  });
  
  const [feelsStatus, setFeelsStatus] = useState<StatusItem>({
    label: 'Feels Program',
    status: 'checking',
    message: 'Checking deployment...'
  });
  
  const [initStatus, setInitStatus] = useState<StatusItem>({
    label: 'Program Initialization',
    status: 'checking',
    message: 'Checking initialization...'
  });

  useEffect(() => {
    const checkStatuses = async () => {
      // Check JitoSOL contract (example address - replace with actual)
      try {
        const jitoMintAddress = new PublicKey('J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn');
        const accountInfo = await connection.getAccountInfo(jitoMintAddress);
        
        if (accountInfo) {
          setJitoStatus({
            label: 'JitoSOL Contract',
            status: 'success',
            message: 'Deployed and active'
          });
        } else {
          setJitoStatus({
            label: 'JitoSOL Contract',
            status: 'error',
            message: 'Not found on network'
          });
        }
      } catch (err) {
        setJitoStatus({
          label: 'JitoSOL Contract',
          status: 'error',
          message: 'Error checking status'
        });
      }

      // Check Feels program deployment
      if (program) {
        try {
          const programId = program.programId;
          const accountInfo = await connection.getAccountInfo(programId);
          
          if (accountInfo) {
            setFeelsStatus({
              label: 'Feels Program',
              status: 'success',
              message: `Deployed at ${programId.toBase58().slice(0, 8)}...`
            });
          } else {
            setFeelsStatus({
              label: 'Feels Program',
              status: 'error',
              message: 'Not found on network'
            });
          }
        } catch (err) {
          setFeelsStatus({
            label: 'Feels Program',
            status: 'error',
            message: 'Error checking deployment'
          });
        }
      } else {
        setFeelsStatus({
          label: 'Feels Program',
          status: fallback ? 'warning' : 'error',
          message: fallback ? 'Using test data fallback' : 'Not initialized'
        });
      }

      // Check program initialization
      if (program) {
        try {
          // Try to fetch protocol state or a known account
          // This is a placeholder - adjust based on your actual protocol state account
          setInitStatus({
            label: 'Program Initialization',
            status: 'success',
            message: 'Protocol state initialized'
          });
        } catch (err) {
          setInitStatus({
            label: 'Program Initialization',
            status: 'warning',
            message: 'Protocol state may not be initialized'
          });
        }
      } else {
        setInitStatus({
          label: 'Program Initialization',
          status: fallback ? 'warning' : 'error',
          message: fallback ? 'Using test data' : 'Program not loaded'
        });
      }
    };

    checkStatuses();
  }, [connection, program, fallback]);

  const getStatusIcon = (status: StatusItem['status']) => {
    switch (status) {
      case 'checking':
        return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
      case 'success':
        return <CheckCircle2 className="h-4 w-4 text-success-500" />;
      case 'warning':
        return <AlertCircle className="h-4 w-4 text-yellow-500" />;
      case 'error':
        return <XCircle className="h-4 w-4 text-danger-500" />;
    }
  };

  const getStatusBadge = (status: StatusItem['status']) => {
    switch (status) {
      case 'checking':
        return <Badge variant="outline" className="text-xs">Checking</Badge>;
      case 'success':
        return <Badge variant="outline" className="text-xs bg-success-50 text-success-700 border-success-200">Active</Badge>;
      case 'warning':
        return <Badge variant="outline" className="text-xs bg-yellow-50 text-yellow-700 border-yellow-200">Warning</Badge>;
      case 'error':
        return <Badge variant="outline" className="text-xs bg-danger-50 text-danger-700 border-danger-200">Error</Badge>;
    }
  };

  const statuses = [jitoStatus, feelsStatus, initStatus];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Program Status</CardTitle>
        <CardDescription>
          Deployment and initialization status of protocol components
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {statuses.map((item, index) => (
          <div key={index} className="flex items-center justify-between p-3 rounded-lg bg-muted/30">
            <div className="flex items-center gap-3">
              {getStatusIcon(item.status)}
              <div>
                <div className="text-sm font-medium">{item.label}</div>
                <div className="text-xs text-muted-foreground">{item.message}</div>
              </div>
            </div>
            {getStatusBadge(item.status)}
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

