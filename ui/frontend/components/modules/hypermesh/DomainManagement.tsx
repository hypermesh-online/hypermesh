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
  useDomainList,
  useRegisterDomain,
  useJoinDomain,
} from '@/lib/hooks/useBlockMatrix';
import {
  Globe,
  AlertTriangle,
  Plus,
  LogIn,
  Network,
} from 'lucide-react';

export function DomainManagement() {
  const { data: domains, isLoading, error } = useDomainList();
  const registerDomain = useRegisterDomain();
  const joinDomain = useJoinDomain();

  const [regName, setRegName] = React.useState('');
  const [joinName, setJoinName] = React.useState('');
  const [joinToken, setJoinToken] = React.useState('');

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

  const handleRegister = () => {
    const name = regName.trim();
    if (name) {
      registerDomain.mutate({ name }, {
        onSuccess: () => setRegName(''),
      });
    }
  };

  const handleJoin = () => {
    const domain = joinName.trim();
    const token = joinToken.trim();
    if (domain && token) {
      joinDomain.mutate({ domain, token }, {
        onSuccess: () => {
          setJoinName('');
          setJoinToken('');
        },
      });
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Domain Management</h2>

      {/* Domain list */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Network className="h-5 w-5 text-cyan-400" />
            Registered Domains
          </CardTitle>
          <CardDescription className="text-gray-400">
            Domains create Network-scope blockchains ({domains?.length ?? 0} domains)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {domains && domains.length > 0 ? (
            <div className="space-y-2 max-h-80 overflow-y-auto">
              {domains.map((domain) => (
                <div
                  key={domain.name}
                  className="flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5"
                >
                  <div className="flex items-center gap-3">
                    <Globe className="h-4 w-4 text-cyan-400" />
                    <div>
                      <p className="text-sm text-white font-medium">{domain.name}</p>
                      <p className="text-xs text-gray-400 font-mono">
                        Owner: {domain.owner.slice(0, 16)}...
                      </p>
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
              <p>No domains registered</p>
              <p className="text-xs text-gray-500 mt-1">
                Register a domain to create a Network-scope blockchain
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Register domain */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Plus className="h-5 w-5 text-cyan-400" />
            Register Domain
          </CardTitle>
          <CardDescription className="text-gray-400">
            Create a new domain with its own Network-scope blockchain
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Input
            placeholder="Domain name (e.g., mynetwork)"
            value={regName}
            onChange={(e) => setRegName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleRegister()}
            className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
          />
          <Button
            onClick={handleRegister}
            disabled={!regName.trim() || registerDomain.isPending}
            className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
          >
            {registerDomain.isPending ? 'Registering...' : 'Register Domain'}
          </Button>
          {registerDomain.isError && (
            <p className="text-sm text-red-400">{registerDomain.error.message}</p>
          )}
          {registerDomain.isSuccess && (
            <p className="text-sm text-green-400">Domain registered successfully.</p>
          )}
        </CardContent>
      </Card>

      {/* Join domain */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <LogIn className="h-5 w-5 text-cyan-400" />
            Join Domain
          </CardTitle>
          <CardDescription className="text-gray-400">
            Join an existing domain using a BLAKE3-HMAC invitation token
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Input
            placeholder="Domain name"
            value={joinName}
            onChange={(e) => setJoinName(e.target.value)}
            className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
          />
          <Input
            placeholder="Invitation token"
            value={joinToken}
            onChange={(e) => setJoinToken(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleJoin()}
            className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
          />
          <Button
            onClick={handleJoin}
            disabled={!joinName.trim() || !joinToken.trim() || joinDomain.isPending}
            className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
          >
            {joinDomain.isPending ? 'Joining...' : 'Join Domain'}
          </Button>
          {joinDomain.isError && (
            <p className="text-sm text-red-400">{joinDomain.error.message}</p>
          )}
          {joinDomain.isSuccess && (
            <p className="text-sm text-green-400">Successfully joined domain.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
