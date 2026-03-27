// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { ModuleLoading } from '@/components/ui/ModuleLoading';
import {
  useDnsList,
  useDnsResolve,
  useRegisterDns,
} from '@/lib/hooks/useBlockMatrix';
import {
  Globe,
  AlertTriangle,
  Search,
  Plus,
  Server,
} from 'lucide-react';

export function DnsManagement() {
  const { data: records, isLoading, error } = useDnsList();
  const registerDns = useRegisterDns();

  const [resolveName, setResolveName] = React.useState<string | undefined>(undefined);
  const [resolveInput, setResolveInput] = React.useState('');
  const [regName, setRegName] = React.useState('');
  const [regAddress, setRegAddress] = React.useState('');

  const { data: resolved, isLoading: resolving, error: resolveError } = useDnsResolve(resolveName);

  if (isLoading) return <ModuleLoading />;

  if (error) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{error.message}</p>
        </CardContent>
      </Card>
    );
  }

  const handleResolve = () => {
    const name = resolveInput.trim();
    if (name) setResolveName(name);
  };

  const handleRegister = () => {
    const name = regName.trim();
    const address = regAddress.trim();
    if (name && address) {
      registerDns.mutate(
        { name, address },
        {
          onSuccess: () => {
            setRegName('');
            setRegAddress('');
          },
        },
      );
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">DNS Management</h2>

      {/* DNS records table */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Globe className="h-5 w-5 text-cyan-400" />
            DNS Records
          </CardTitle>
          <CardDescription className="text-gray-400">
            Blockchain-registered DNS names ({records?.length ?? 0} records)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {records && records.length > 0 ? (
            <div className="space-y-2 max-h-80 overflow-y-auto">
              {records.map((record) => (
                <div
                  key={record.name}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3">
                    <Server className="h-4 w-4 text-cyan-400" />
                    <div>
                      <p className="text-sm text-white font-medium">{record.name}</p>
                      <p className="text-xs text-gray-400 font-mono">{record.address}</p>
                    </div>
                  </div>
                  <Badge variant="outline" className="text-xs bg-green-500/20 text-green-400 border-green-500/30">
                    Active
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-6 text-gray-400">
              <Globe className="h-10 w-10 text-gray-600 mx-auto mb-2" />
              <p>No DNS records registered</p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Resolve lookup */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Search className="h-5 w-5 text-cyan-400" />
            Resolve DNS Name
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex gap-2">
            <Input
              placeholder="Enter DNS name to resolve..."
              value={resolveInput}
              onChange={(e) => setResolveInput(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleResolve()}
              className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
            />
            <Button
              onClick={handleResolve}
              variant="outline"
              className="border-cyan-500/30 text-cyan-400"
              disabled={!resolveInput.trim()}
            >
              Resolve
            </Button>
          </div>
          {resolveName && (
            <div className="p-3 rounded-lg bg-cyan-500/5 border border-cyan-500/20">
              {resolving ? (
                <p className="text-sm text-gray-400">Resolving...</p>
              ) : resolveError ? (
                <p className="text-sm text-gray-500">Name not found</p>
              ) : resolved ? (
                <div className="text-sm">
                  <span className="text-gray-400">Result: </span>
                  <span className="text-cyan-400 font-mono">{resolved.address}</span>
                </div>
              ) : null}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Register form */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Plus className="h-5 w-5 text-cyan-400" />
            Register DNS Record
          </CardTitle>
          <CardDescription className="text-gray-400">
            Register a new name on the blockchain (requires Proof of State)
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Input
            placeholder="Name (e.g., mynode)"
            value={regName}
            onChange={(e) => setRegName(e.target.value)}
            className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
          />
          <Input
            placeholder="Address (e.g., [::1]:9292)"
            value={regAddress}
            onChange={(e) => setRegAddress(e.target.value)}
            className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
          />
          <Button
            onClick={handleRegister}
            disabled={!regName.trim() || !regAddress.trim() || registerDns.isPending}
            className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
          >
            {registerDns.isPending ? 'Registering...' : 'Register'}
          </Button>
          {registerDns.isError && (
            <p className="text-sm text-red-400">{registerDns.error.message}</p>
          )}
          {registerDns.isSuccess && (
            <p className="text-sm text-green-400">DNS record registered successfully.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
