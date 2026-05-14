// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Search, Filter } from 'lucide-react';
import { type PrivacyLevel } from '@/lib/types';

interface CatalogSearchHeaderProps {
  selectedPrivacyLevel: PrivacyLevel;
  onPrivacyLevelChange: (level: PrivacyLevel) => void;
}

export function CatalogSearchHeader({ selectedPrivacyLevel, onPrivacyLevelChange }: CatalogSearchHeaderProps) {
  return (
    <>
      {/* Header */}
      <div className="text-center py-6">
        <h1 className="text-3xl font-bold bg-gradient-to-r from-purple-400 to-pink-600 bg-clip-text text-transparent mb-2">
          HyperMesh Asset Catalog
        </h1>
        <p className="text-gray-400 max-w-2xl mx-auto">
          Browse, install, and execute applications through HyperMesh asset management with Proof of State verification and NAT-like proxy addressing.
        </p>
      </div>

      {/* Search and Privacy Controls */}
      <Card className="bg-black/40 border-purple-500/30 backdrop-blur-lg">
        <CardContent className="p-4">
          <div className="flex gap-4 items-center">
            <div className="flex-1 relative">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
              <input
                type="text"
                placeholder="Search HyperMesh assets..."
                className="w-full pl-10 pr-4 py-2 bg-black/40 border border-purple-500/30 rounded text-white placeholder:text-gray-400"
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="text-sm text-gray-400">Privacy Level:</label>
              <select
                value={selectedPrivacyLevel}
                onChange={(e) => onPrivacyLevelChange(e.target.value as PrivacyLevel)}
                className="bg-black/40 border border-purple-500/30 rounded px-3 py-2 text-white text-sm"
              >
                <option value="private">Private (Internal only)</option>
                <option value="federated">Federated (Trusted networks)</option>
                <option value="public">Public (Cross-network)</option>
                <option value="anonymous">Anonymous (Privacy-first)</option>
                <option value="verified">Verified (Full state proof)</option>
              </select>
            </div>
            <Button 
              variant="outline" 
              size="sm"
              className="border-purple-500/30 text-purple-400 hover:bg-purple-500/10"
            >
              <Filter className="h-4 w-4 mr-2" />
              Filters
            </Button>
          </div>
        </CardContent>
      </Card>
    </>
  );
}