// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { 
  Package, 
  Download,
  Play,
  CheckCircle,
  AlertTriangle,
  Clock,
  Eye,
  Cpu,
  Database,
  Globe,
  Shield,
  Activity,
  Zap
} from 'lucide-react';
import type { CatalogApplication } from '@/lib/api';

interface ApplicationCardProps {
  app: CatalogApplication;
  onInstall: (app: CatalogApplication) => Promise<void>;
  onRun: (app: CatalogApplication) => Promise<void>;
  isInstalling: boolean;
  isRunning: boolean;
}

export function ApplicationCard({ app, onInstall, onRun, isInstalling, isRunning }: ApplicationCardProps) {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Installing': return <Clock className="h-3 w-3 mr-1 animate-spin" />;
      case 'Failed': return <AlertTriangle className="h-3 w-3 mr-1" />;
      case 'Installed': return <CheckCircle className="h-3 w-3 mr-1" />;
      default: return null;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Installed': return 'bg-green-500/20 text-green-400 border-green-500/30';
      case 'Installing': return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      case 'Failed': return 'bg-red-500/20 text-red-400 border-red-500/30';
      default: return 'bg-purple-500/20 text-purple-400 border-purple-500/30';
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'ai': return <Zap className="h-4 w-4" />;
      case 'database': return <Database className="h-4 w-4" />;
      case 'web': return <Globe className="h-4 w-4" />;
      case 'compute': return <Cpu className="h-4 w-4" />;
      case 'security': return <Shield className="h-4 w-4" />;
      default: return <Package className="h-4 w-4" />;
    }
  };

  return (
    <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg hover:border-purple-400/50 transition-all">
      <CardHeader className="pb-2">
        <div className="flex justify-between items-start">
          <div className="flex items-center gap-2">
            {getCategoryIcon(app.type)}
            <CardTitle className="text-lg text-white">{app.name}</CardTitle>
          </div>
          <Badge className={`text-xs border ${getStatusColor(app.status)}`}>
            {getStatusIcon(app.status)}
            {app.status}
          </Badge>
        </div>
        <CardDescription className="text-gray-400 text-sm">
          {app.description}
        </CardDescription>
      </CardHeader>
      
      <CardContent className="space-y-4">
        {/* Resource Requirements */}
        <div className="space-y-2">
          <div className="flex justify-between text-xs">
            <span className="text-gray-400">CPU</span>
            <span className="text-purple-400">{app.requirements.cpu} cores</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-gray-400">Memory</span>
            <span className="text-purple-400">{app.requirements.memory}GB</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-gray-400">Storage</span>
            <span className="text-purple-400">{app.requirements.storage}GB</span>
          </div>
        </div>

        {/* VM Asset Info */}
        {app.assetId && (
          <div className="p-2 bg-green-500/10 border border-green-500/30 rounded text-xs">
            <div className="flex items-center gap-1 text-green-400 mb-1">
              <CheckCircle className="h-3 w-3" />
              VM Asset Created
            </div>
            <div className="text-gray-400">Asset ID: {app.assetId}</div>
          </div>
        )}

        {/* Rating */}
        {app.rating > 0 && (
          <div className="space-y-2">
            <div className="flex justify-between text-xs">
              <span className="text-gray-400">Rating</span>
              <span className="text-green-400">{app.rating}/5</span>
            </div>
            <Progress
              value={Math.min((app.rating / 5) * 100, 100)}
              className="h-1"
            />
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex gap-2 pt-2">
          {!app.assetId ? (
            <Button 
              onClick={() => onInstall(app)}
              disabled={isInstalling}
              className="flex-1 bg-purple-600 hover:bg-purple-700 text-white text-sm"
            >
              <Download className="h-3 w-3 mr-1" />
              {isInstalling ? 'Installing...' : 'Install as VM'}
            </Button>
          ) : (
            <Button 
              onClick={() => onRun(app)}
              disabled={isRunning}
              className="flex-1 bg-green-600 hover:bg-green-700 text-white text-sm"
            >
              <Play className="h-3 w-3 mr-1" />
              {isRunning ? 'Starting...' : 'Execute'}
            </Button>
          )}
          
          <Button 
            variant="outline" 
            size="sm"
            className="border-purple-500/30 text-purple-400 hover:bg-purple-500/10"
          >
            <Eye className="h-3 w-3" />
          </Button>
        </div>

        {/* Dependencies */}
        {app.dependencies && app.dependencies.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {app.dependencies.slice(0, 3).map((dep: string) => (
              <Badge
                key={dep}
                variant="secondary"
                className="text-xs bg-purple-500/20 text-purple-300 border-purple-500/30"
              >
                {dep}
              </Badge>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}