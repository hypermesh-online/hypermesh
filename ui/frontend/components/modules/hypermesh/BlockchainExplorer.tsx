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
  useBlockchainHeight,
  useBlock,
  useChainValidation,
} from '@/lib/hooks/useBlockMatrix';
import {
  Database,
  AlertTriangle,
  CheckCircle,
  Search,
  ChevronLeft,
  ChevronRight,
  Hash,
} from 'lucide-react';

export function BlockchainExplorer() {
  const { data: heightData, isLoading: heightLoading, error: heightError } = useBlockchainHeight();
  const { data: chainValid, isLoading: validating, refetch: revalidate } = useChainValidation();

  const [selectedIndex, setSelectedIndex] = React.useState<number | undefined>(undefined);
  const [searchInput, setSearchInput] = React.useState('');

  const { data: blockData, isLoading: blockLoading, error: blockError } = useBlock(selectedIndex);

  const height = heightData?.height ?? 0;

  if (heightLoading) return <ModuleLoading />;

  if (heightError) {
    return (
      <Card className="m-4 border-red-500/30">
        <CardContent className="p-6 text-center">
          <AlertTriangle className="h-8 w-8 text-red-400 mx-auto mb-2" />
          <p className="text-red-400">{heightError.message}</p>
        </CardContent>
      </Card>
    );
  }

  const handleSearch = () => {
    const idx = parseInt(searchInput, 10);
    if (!isNaN(idx) && idx >= 0 && idx < height) {
      setSelectedIndex(idx);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSearch();
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Blockchain Explorer</h2>

      {/* Chain overview */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Chain Height</CardTitle>
            <Database className="h-4 w-4 text-cyan-400" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-cyan-400">{height}</div>
            <p className="text-xs text-gray-400">Total blocks</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium text-white">Chain Valid</CardTitle>
            {chainValid?.valid ? (
              <CheckCircle className="h-4 w-4 text-green-400" />
            ) : (
              <AlertTriangle className="h-4 w-4 text-yellow-400" />
            )}
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${chainValid?.valid ? 'text-green-400' : 'text-yellow-400'}`}>
              {validating ? '...' : chainValid?.valid ? 'Yes' : 'Unknown'}
            </div>
            <p className="text-xs text-gray-400">Integrity check</p>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg flex items-center">
          <CardContent className="w-full py-4">
            <Button
              onClick={() => revalidate()}
              disabled={validating}
              className="w-full bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-black"
            >
              {validating ? 'Validating...' : 'Validate Chain'}
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Block search */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Search className="h-5 w-5 text-cyan-400" />
            Block Lookup
          </CardTitle>
          <CardDescription className="text-gray-400">
            Enter a block index (0 to {height - 1}) to view its details
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex gap-2">
            <Input
              type="number"
              min={0}
              max={height - 1}
              placeholder="Block index..."
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              onKeyDown={handleKeyDown}
              className="bg-black/30 border-cyan-500/20 text-white placeholder:text-gray-500"
            />
            <Button
              onClick={handleSearch}
              className="border-cyan-500/30 text-cyan-400"
              variant="outline"
            >
              <Search className="h-4 w-4" />
            </Button>
          </div>

          {/* Navigation */}
          {selectedIndex !== undefined && (
            <div className="flex items-center gap-2 mt-3">
              <Button
                size="sm"
                variant="outline"
                className="border-cyan-500/30 text-cyan-400"
                disabled={selectedIndex <= 0}
                onClick={() => setSelectedIndex(Math.max(0, selectedIndex - 1))}
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <span className="text-sm text-gray-400">
                Block #{selectedIndex}
              </span>
              <Button
                size="sm"
                variant="outline"
                className="border-cyan-500/30 text-cyan-400"
                disabled={selectedIndex >= height - 1}
                onClick={() => setSelectedIndex(Math.min(height - 1, selectedIndex + 1))}
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Block detail */}
      {selectedIndex !== undefined && (
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Hash className="h-5 w-5 text-cyan-400" />
              Block #{selectedIndex}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {blockLoading ? (
              <div className="space-y-2">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="animate-pulse h-6 bg-gray-700 rounded" />
                ))}
              </div>
            ) : blockError ? (
              <p className="text-red-400 text-sm">{blockError.message}</p>
            ) : blockData ? (
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-gray-400">Index</span>
                  <span className="text-white font-mono">{blockData.index}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Hash</span>
                  <span className="text-cyan-400 font-mono text-sm truncate ml-4 max-w-[300px]">
                    {blockData.hash}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Previous Hash</span>
                  <span className="text-gray-300 font-mono text-sm truncate ml-4 max-w-[300px]">
                    {blockData.previous_hash}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Timestamp</span>
                  <span className="text-white font-mono">
                    {new Date(blockData.timestamp * 1000).toLocaleString()}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-400">Assets</span>
                  <Badge variant="outline" className="text-xs bg-purple-500/20 text-purple-400 border-purple-500/30">
                    {blockData.assets}
                  </Badge>
                </div>
              </div>
            ) : null}
          </CardContent>
        </Card>
      )}

      {/* Recent blocks list */}
      {height > 0 && selectedIndex === undefined && (
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white">Recent Blocks</CardTitle>
            <CardDescription className="text-gray-400">
              Click a block to view its details
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {Array.from({ length: Math.min(10, height) }, (_, i) => height - 1 - i).map((idx) => (
                <button
                  key={idx}
                  onClick={() => setSelectedIndex(idx)}
                  className="w-full flex items-center justify-between p-3 border border-cyan-500/20 rounded-lg bg-cyan-500/5 hover:bg-cyan-500/10 transition-colors text-left"
                >
                  <div className="flex items-center gap-3">
                    <Database className="h-4 w-4 text-cyan-400" />
                    <span className="text-white font-mono text-sm">Block #{idx}</span>
                  </div>
                  <ChevronRight className="h-4 w-4 text-gray-400" />
                </button>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
