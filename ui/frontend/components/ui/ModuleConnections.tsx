// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ArrowRight, Network, Shield, Zap, Coins, Server, Gauge } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Link } from 'react-router-dom';

interface ModuleConnection {
  name: string;
  icon: React.ComponentType<{ className?: string }>;
  status: 'enabled' | 'available' | 'locked';
  description: string;
  href: string;
  dependencies?: string[];
  provides?: string[];
}

interface ModuleConnectionsProps {
  currentModule: string;
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  className?: string;
}

export function ModuleConnections({
  currentModule,
  theme = 'cyan',
  className
}: ModuleConnectionsProps) {
  const getThemeColors = () => {
    const themes = {
      cyan: {
        border: 'border-cyan-500/30',
        bg: 'bg-cyan-500/5',
        accent: 'bg-cyan-500/10 border-cyan-500/20'
      },
      green: {
        border: 'border-green-500/30',
        bg: 'bg-green-500/5',
        accent: 'bg-green-500/10 border-green-500/20'
      },
      purple: {
        border: 'border-purple-500/30',
        bg: 'bg-purple-500/5',
        accent: 'bg-purple-500/10 border-purple-500/20'
      },
      red: {
        border: 'border-red-500/30',
        bg: 'bg-red-500/5',
        accent: 'bg-red-500/10 border-red-500/20'
      },
      yellow: {
        border: 'border-yellow-500/30',
        bg: 'bg-yellow-500/5',
        accent: 'bg-yellow-500/10 border-yellow-500/20'
      }
    };
    return themes[theme];
  };

  const getModuleConnections = (): ModuleConnection[] => {
    const allModules: ModuleConnection[] = [
      {
        name: 'TrustChain',
        icon: Shield,
        status: 'enabled',
        description: 'Foundation layer for identity and trust',
        href: '/trustchain',
        provides: ['Identity verification', 'Trust relationships', 'Network access control']
      },
      {
        name: 'STOQ',
        icon: Zap,
        status: 'enabled',
        description: 'High-performance transport protocol',
        href: '/stoq',
        dependencies: ['TrustChain identity'],
        provides: ['P2P tunneling', 'High-speed transport', 'Protocol optimization']
      },
      {
        name: 'HyperMesh',
        icon: Network,
        status: 'available',
        description: 'Multi-network resource management',
        href: '/hypermesh',
        dependencies: ['TrustChain identity', 'STOQ transport'],
        provides: ['Resource distribution', 'Multi-network interface', 'Permission management']
      },
      {
        name: 'Caesar',
        icon: Coins,
        status: 'available',
        description: 'Economic system and tokenization',
        href: '/caesar',
        dependencies: ['HyperMesh access'],
        provides: ['CAESAR tokens', 'Economic participation', 'Governance voting']
      },
      {
        name: 'Catalog',
        icon: Server,
        status: 'locked',
        description: 'Asset creation and management',
        href: '/catalog',
        dependencies: ['HyperMesh access', 'Economic participation'],
        provides: ['Asset registry', 'Resource creation', 'Service deployment']
      },
      {
        name: 'NGauge',
        icon: Gauge,
        status: 'enabled',
        description: 'Analytics and resource marketplace',
        href: '/ngauge',
        dependencies: ['STOQ transport', 'BlockMatrix metrics'],
        provides: ['Capacity metrics', 'Routing intelligence', 'Marketplace']
      }
    ];

    return allModules.filter(module => module.name.toLowerCase() !== currentModule.toLowerCase());
  };

  const getStatusBadge = (status: ModuleConnection['status']) => {
    switch (status) {
      case 'enabled':
        return <Badge className="bg-green-500/20 text-green-400 border-green-500/30">Enabled</Badge>;
      case 'available':
        return <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30">Available</Badge>;
      case 'locked':
        return <Badge className="bg-gray-500/20 text-gray-400 border-gray-500/30">Locked</Badge>;
    }
  };

  const colors = getThemeColors();
  const connections = getModuleConnections();

  if (connections.length === 0) {
    return null;
  }

  return (
    <Card className={cn(
      'backdrop-blur-lg',
      colors.border,
      colors.bg,
      className
    )}>
      <CardHeader>
        <CardTitle className="text-white flex items-center gap-2">
          <Network className="h-5 w-5 text-cyan-400" />
          Connected Modules
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {connections.map((connection) => {
            const Icon = connection.icon;
            const isAccessible = connection.status === 'enabled' || connection.status === 'available';

            return (
              <div key={connection.name} className={cn(
                'p-4 rounded-lg border transition-all duration-200',
                colors.accent,
                isAccessible && 'hover:shadow-lg hover:scale-[1.02]'
              )}>
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <Icon className="h-6 w-6 text-gray-400" />
                    <div>
                      <h4 className="font-medium text-white">{connection.name}</h4>
                      <p className="text-sm text-gray-400">{connection.description}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {getStatusBadge(connection.status)}
                    {isAccessible && (
                      <Link to={connection.href}>
                        <Button size="sm" variant="outline" className="border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/20">
                          <ArrowRight className="h-4 w-4" />
                        </Button>
                      </Link>
                    )}
                  </div>
                </div>

                {connection.dependencies && (
                  <div className="mb-2">
                    <p className="text-xs font-medium text-gray-300 mb-1">Requires:</p>
                    <div className="flex flex-wrap gap-1">
                      {connection.dependencies.map((dep, index) => (
                        <Badge key={index} variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
                          {dep}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {connection.provides && (
                  <div>
                    <p className="text-xs font-medium text-gray-300 mb-1">Provides:</p>
                    <div className="flex flex-wrap gap-1">
                      {connection.provides.map((feature, index) => (
                        <Badge key={index} variant="outline" className="text-xs bg-blue-500/20 text-blue-400 border-blue-500/30">
                          {feature}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
