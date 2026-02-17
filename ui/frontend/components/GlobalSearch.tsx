// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect, useRef } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Search,
  Filter,
  History,
  Star,
  X,
  FileText,
  Network,
  Shield,
  Server,
  Coins,
  Clock,
  ChevronDown,
  BookmarkIcon,
  TrendingUp,
  Globe,
  Loader2
} from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  useSearch,
  useSearchSuggestions,
  useRecentSearches,
  useTrendingSearches,
  SearchResult,
  SearchFilter
} from '@/lib/api/hooks/useSearch';

interface GlobalSearchProps {
  onResultSelect?: (result: SearchResult) => void;
  className?: string;
}

export function GlobalSearch({ onResultSelect, className }: GlobalSearchProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [inputValue, setInputValue] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const [selectedResultIndex, setSelectedResultIndex] = useState(-1);

  const searchRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Real-time search with backend
  const {
    query,
    setQuery,
    filters,
    setFilters,
    results,
    total,
    isLoading,
    error
  } = useSearch();

  // Get suggestions, recent searches, and trending
  const suggestions = useSearchSuggestions(inputValue);
  const recentSearches = useRecentSearches();
  const trendingSearches = useTrendingSearches();

  // Update query with debouncing
  useEffect(() => {
    const timer = setTimeout(() => {
      setQuery(inputValue);
    }, 300);
    return () => clearTimeout(timer);
  }, [inputValue, setQuery]);

  // Handle outside clicks
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (searchRef.current && !searchRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Keyboard navigation
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!isOpen) return;

      switch (event.key) {
        case 'ArrowDown':
          event.preventDefault();
          setSelectedResultIndex(prev =>
            prev < results.length - 1 ? prev + 1 : prev
          );
          break;
        case 'ArrowUp':
          event.preventDefault();
          setSelectedResultIndex(prev => prev > 0 ? prev - 1 : -1);
          break;
        case 'Enter':
          event.preventDefault();
          if (selectedResultIndex >= 0 && results[selectedResultIndex]) {
            handleResultSelect(results[selectedResultIndex]);
          }
          break;
        case 'Escape':
          setIsOpen(false);
          inputRef.current?.blur();
          break;
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, selectedResultIndex, results]);

  const handleResultSelect = (result: SearchResult) => {
    onResultSelect?.(result);
    setIsOpen(false);
    setInputValue('');
    setQuery('');
  };

  const handleFilterChange = (key: keyof SearchFilter, value: any) => {
    setFilters(prev => ({
      ...prev,
      [key]: value
    }));
  };

  const getTypeIcon = (type: SearchResult['type']) => {
    switch (type) {
      case 'asset':
        return <Server className="h-4 w-4" />;
      case 'transaction':
        return <Coins className="h-4 w-4" />;
      case 'certificate':
        return <Shield className="h-4 w-4" />;
      case 'node':
        return <Network className="h-4 w-4" />;
      case 'contract':
        return <FileText className="h-4 w-4" />;
      case 'compute-job':
        return <Globe className="h-4 w-4" />;
      default:
        return <Search className="h-4 w-4" />;
    }
  };

  const getTypeColor = (type: SearchResult['type']) => {
    switch (type) {
      case 'asset':
        return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
      case 'transaction':
        return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30';
      case 'certificate':
        return 'bg-green-500/20 text-green-400 border-green-500/30';
      case 'node':
        return 'bg-purple-500/20 text-purple-400 border-purple-500/30';
      case 'contract':
        return 'bg-orange-500/20 text-orange-400 border-orange-500/30';
      case 'compute-job':
        return 'bg-cyan-500/20 text-cyan-400 border-cyan-500/30';
      default:
        return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
    }
  };

  return (
    <div ref={searchRef} className={cn("relative w-full max-w-2xl", className)}>
      {/* Search Input */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
        <Input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onFocus={() => setIsOpen(true)}
          placeholder="Search assets, transactions, certificates, nodes..."
          className="pl-10 pr-20 h-10 bg-black/40 border-gray-800 text-white placeholder-gray-500 focus:border-purple-500 transition-colors"
        />
        <div className="absolute right-2 top-1/2 transform -translate-y-1/2 flex items-center space-x-2">
          {isLoading && (
            <Loader2 className="h-4 w-4 animate-spin text-purple-400" />
          )}
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setShowFilters(!showFilters)}
            className="h-6 px-2 text-gray-400 hover:text-white"
          >
            <Filter className="h-3 w-3 mr-1" />
            Filters
          </Button>
          {inputValue && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                setInputValue('');
                setQuery('');
              }}
              className="h-6 px-1 text-gray-400 hover:text-white"
            >
              <X className="h-3 w-3" />
            </Button>
          )}
        </div>
      </div>

      {/* Search Dropdown */}
      {isOpen && (
        <Card className="absolute top-12 left-0 right-0 z-50 bg-black/95 border-gray-800 backdrop-blur-xl shadow-2xl max-h-[600px] overflow-hidden">
          <CardContent className="p-0">
            {/* Filters */}
            {showFilters && (
              <div className="p-4 border-b border-gray-800">
                <div className="space-y-3">
                  <div>
                    <label className="text-xs font-medium text-gray-400 mb-1 block">Type</label>
                    <div className="flex flex-wrap gap-2">
                      {['asset', 'transaction', 'certificate', 'node', 'contract', 'compute-job'].map(type => (
                        <Badge
                          key={type}
                          variant="outline"
                          className={cn(
                            "cursor-pointer transition-colors",
                            filters.type?.includes(type)
                              ? getTypeColor(type as SearchResult['type'])
                              : "bg-black/40 text-gray-400 border-gray-700 hover:border-gray-600"
                          )}
                          onClick={() => {
                            const currentTypes = filters.type || [];
                            if (currentTypes.includes(type)) {
                              handleFilterChange('type', currentTypes.filter(t => t !== type));
                            } else {
                              handleFilterChange('type', [...currentTypes, type]);
                            }
                          }}
                        >
                          {type}
                        </Badge>
                      ))}
                    </div>
                  </div>

                  <div>
                    <label className="text-xs font-medium text-gray-400 mb-1 block">Network</label>
                    <div className="flex flex-wrap gap-2">
                      {['HyperMesh', 'Caesar', 'TrustChain', 'Catalog'].map(network => (
                        <Badge
                          key={network}
                          variant="outline"
                          className={cn(
                            "cursor-pointer transition-colors",
                            filters.network?.includes(network.toLowerCase())
                              ? "bg-purple-500/20 text-purple-400 border-purple-500/30"
                              : "bg-black/40 text-gray-400 border-gray-700 hover:border-gray-600"
                          )}
                          onClick={() => {
                            const currentNetworks = filters.network || [];
                            const networkLower = network.toLowerCase();
                            if (currentNetworks.includes(networkLower)) {
                              handleFilterChange('network', currentNetworks.filter(n => n !== networkLower));
                            } else {
                              handleFilterChange('network', [...currentNetworks, networkLower]);
                            }
                          }}
                        >
                          {network}
                        </Badge>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Search Results */}
            {query ? (
              <div className="overflow-y-auto max-h-[400px]">
                {isLoading ? (
                  <div className="p-4 space-y-3">
                    {[1, 2, 3].map(i => (
                      <div key={i} className="space-y-2">
                        <Skeleton className="h-4 w-3/4" />
                        <Skeleton className="h-3 w-1/2" />
                      </div>
                    ))}
                  </div>
                ) : error ? (
                  <div className="p-4 text-center text-red-400">
                    Search failed. Please try again.
                  </div>
                ) : results.length === 0 ? (
                  <div className="p-8 text-center">
                    <p className="text-gray-400">No results found for "{query}"</p>
                    <p className="text-xs text-gray-500 mt-2">Try adjusting your filters or search terms</p>
                  </div>
                ) : (
                  <div>
                    <div className="px-4 py-2 bg-black/60 border-b border-gray-800">
                      <p className="text-xs text-gray-400">
                        Found {total} results for "{query}"
                      </p>
                    </div>
                    {results.map((result, index) => (
                      <div
                        key={result.id}
                        className={cn(
                          "px-4 py-3 hover:bg-gray-900/50 cursor-pointer transition-colors border-b border-gray-800/50",
                          selectedResultIndex === index && "bg-gray-900/50"
                        )}
                        onClick={() => handleResultSelect(result)}
                        onMouseEnter={() => setSelectedResultIndex(index)}
                      >
                        <div className="flex items-start space-x-3">
                          <div className={cn("p-2 rounded-lg", getTypeColor(result.type))}>
                            {getTypeIcon(result.type)}
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-start justify-between">
                              <div className="flex-1">
                                <h3 className="text-sm font-medium text-white truncate">
                                  {result.title}
                                </h3>
                                <p className="text-xs text-gray-400 mt-1 line-clamp-2">
                                  {result.description}
                                </p>
                              </div>
                              {result.relevance && (
                                <Badge variant="outline" className="ml-2 bg-black/40 text-gray-400 border-gray-700">
                                  {result.relevance}%
                                </Badge>
                              )}
                            </div>
                            {result.tags && result.tags.length > 0 && (
                              <div className="flex flex-wrap gap-1 mt-2">
                                {result.tags.slice(0, 3).map(tag => (
                                  <Badge
                                    key={tag}
                                    variant="outline"
                                    className="text-xs bg-black/40 text-gray-500 border-gray-800"
                                  >
                                    {tag}
                                  </Badge>
                                ))}
                              </div>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ) : (
              <div className="p-4">
                {/* Suggestions */}
                {suggestions.data && suggestions.data.length > 0 && (
                  <div className="mb-4">
                    <p className="text-xs font-medium text-gray-400 mb-2">Suggestions</p>
                    <div className="space-y-1">
                      {suggestions.data.map((suggestion, index) => (
                        <div
                          key={index}
                          className="flex items-center justify-between p-2 hover:bg-gray-900/50 rounded-lg cursor-pointer transition-colors"
                          onClick={() => {
                            setInputValue(suggestion.text);
                            setQuery(suggestion.text);
                          }}
                        >
                          <span className="text-sm text-gray-300">{suggestion.text}</span>
                          {suggestion.count && (
                            <span className="text-xs text-gray-500">{suggestion.count} results</span>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Recent Searches */}
                {recentSearches.data && recentSearches.data.length > 0 && (
                  <div className="mb-4">
                    <p className="text-xs font-medium text-gray-400 mb-2 flex items-center">
                      <History className="h-3 w-3 mr-1" />
                      Recent
                    </p>
                    <div className="space-y-1">
                      {recentSearches.data.map((search, index) => (
                        <div
                          key={index}
                          className="flex items-center p-2 hover:bg-gray-900/50 rounded-lg cursor-pointer transition-colors"
                          onClick={() => {
                            setInputValue(search);
                            setQuery(search);
                          }}
                        >
                          <Clock className="h-3 w-3 text-gray-500 mr-2" />
                          <span className="text-sm text-gray-300">{search}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Trending Searches */}
                {trendingSearches.data && trendingSearches.data.length > 0 && (
                  <div>
                    <p className="text-xs font-medium text-gray-400 mb-2 flex items-center">
                      <TrendingUp className="h-3 w-3 mr-1" />
                      Trending
                    </p>
                    <div className="space-y-1">
                      {trendingSearches.data.map((trend, index) => (
                        <div
                          key={index}
                          className="flex items-center justify-between p-2 hover:bg-gray-900/50 rounded-lg cursor-pointer transition-colors"
                          onClick={() => {
                            setInputValue(trend.text);
                            setQuery(trend.text);
                          }}
                        >
                          <span className="text-sm text-gray-300">{trend.text}</span>
                          {trend.count && (
                            <Badge variant="outline" className="bg-purple-500/20 text-purple-400 border-purple-500/30">
                              {trend.count}
                            </Badge>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}