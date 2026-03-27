// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Activity, Settings, User, Globe, Zap, Shield } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Badge } from '@/components/ui/badge';
import { Breadcrumbs } from '@/components/ui/Breadcrumbs';
import { GlobalSearch } from './GlobalSearch';
import { useLocation, useNavigate } from 'react-router-dom';
import { useNodeStatus } from '@/lib/hooks/useBlockMatrix';

export function Header() {
  const location = useLocation();
  const navigate = useNavigate();
  const { data: nodeStatus } = useNodeStatus();

  const getThemeFromPath = () => {
    const path = location.pathname;
    if (path.includes('/trustchain')) return 'green';
    if (path.includes('/caesar')) return 'yellow';
    if (path.includes('/catalog')) return 'red';
    if (path.includes('/engauge')) return 'orange';
    if (path.includes('/stoq')) return 'purple';
    return 'cyan';
  };

  const handleSearchResult = (result: any) => {
    navigate(result.path);
  };

  return (
    <header className="h-16 border-b border-cyan-500/20 bg-black/60 backdrop-blur-lg px-6 flex flex-col">
      <div className="flex items-center justify-between h-16">
        <div className="flex items-center flex-1 max-w-2xl">
          <GlobalSearch onResultSelect={handleSearchResult} className="w-full" />
        </div>
        
        <div className="flex items-center gap-4">
          {nodeStatus && (
            <div className="flex items-center gap-2 px-3 py-1 bg-purple-500/20 border border-purple-500/30 rounded-full">
              <Shield className="h-3 w-3 text-purple-400" />
              <span className="text-xs text-purple-300">{nodeStatus.privacy_mode}</span>
            </div>
          )}
          <div className="flex items-center gap-2 px-3 py-1 bg-green-500/20 border border-green-500/30 rounded-full">
            <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
            <span className="text-xs text-green-300">{nodeStatus ? 'HyperMesh Active' : 'Connecting...'}</span>
          </div>
          
          <Button variant="ghost" size="icon" className="relative text-cyan-400 hover:text-cyan-300 hover:bg-cyan-500/20">
            <Activity className="h-4 w-4" />
            <Badge 
              variant="destructive" 
              className="absolute -top-1 -right-1 h-5 w-5 flex items-center justify-center p-0 text-xs bg-cyan-500 text-black"
            >
              3
            </Badge>
          </Button>
          
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="text-cyan-400 hover:text-cyan-300 hover:bg-cyan-500/20">
                <Globe className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="bg-black/90 border-cyan-500/30">
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Network Status</DropdownMenuItem>
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Protocol Settings</DropdownMenuItem>
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Debug Console</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="text-cyan-400 hover:text-cyan-300 hover:bg-cyan-500/20">
                <User className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="bg-black/90 border-cyan-500/30">
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Node Profile</DropdownMenuItem>
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Identity Management</DropdownMenuItem>
              <DropdownMenuItem className="text-gray-300 hover:text-white hover:bg-cyan-500/20">Disconnect Node</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
      
      {/* Breadcrumbs Row */}
      <div className="pb-3">
        <Breadcrumbs theme={getThemeFromPath() as any} />
      </div>
    </header>
  );
}
