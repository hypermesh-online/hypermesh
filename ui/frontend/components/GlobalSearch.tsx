// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Search } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SearchResult } from '@/lib/types';

interface GlobalSearchProps {
  onResultSelect?: (result: SearchResult) => void;
  className?: string;
}

export function GlobalSearch({ className }: GlobalSearchProps) {
  return (
    <div className={cn('relative w-full max-w-2xl', className)}>
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
        <Input
          type="text"
          disabled
          placeholder="Search is being rebuilt with mesh fan-out (M.4)"
          className="pl-10 pr-4 h-10 bg-black/40 border-gray-800 text-white placeholder-gray-500"
        />
      </div>
      <Card className="absolute top-12 left-0 right-0 z-50 bg-black/95 border-gray-800 backdrop-blur-xl shadow-2xl hidden">
        <CardContent className="p-4 text-sm text-gray-400">
          Search is being rebuilt with mesh fan-out (M.4).
        </CardContent>
      </Card>
    </div>
  );
}
