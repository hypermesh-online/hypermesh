// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Search React Hooks
 *
 * Real-time search integration across all Web3 services
 */

import { useQuery, useMutation } from '@tanstack/react-query';
import { useState, useEffect, useCallback } from 'react';
import {
  searchAPI,
  SearchResult,
  SearchFilter,
  SearchResponse,
  SearchSuggestion
} from '../services/SearchAPI';

// Query keys for cache management
const searchKeys = {
  all: ['search'] as const,
  results: (query: string, filters?: SearchFilter) =>
    [...searchKeys.all, 'results', query, filters] as const,
  suggestions: (query: string) => [...searchKeys.all, 'suggestions', query] as const,
  recent: () => [...searchKeys.all, 'recent'] as const,
  trending: () => [...searchKeys.all, 'trending'] as const,
};

/**
 * Global search hook with debouncing
 */
export function useSearch(initialQuery = '', initialFilters?: SearchFilter) {
  const [query, setQuery] = useState(initialQuery);
  const [filters, setFilters] = useState<SearchFilter>(initialFilters || {});
  const [debouncedQuery, setDebouncedQuery] = useState(query);

  // Debounce search query
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedQuery(query);
    }, 300);

    return () => clearTimeout(timer);
  }, [query]);

  // Search query
  const searchResults = useQuery({
    queryKey: searchKeys.results(debouncedQuery, filters),
    queryFn: () => searchAPI.search(debouncedQuery, filters),
    enabled: debouncedQuery.length > 0,
    staleTime: 30000, // Cache for 30 seconds
  });

  // Save to history when search is performed
  useEffect(() => {
    if (debouncedQuery && searchResults.data) {
      searchAPI.saveSearchToHistory(debouncedQuery);
    }
  }, [debouncedQuery, searchResults.data]);

  return {
    query,
    setQuery,
    filters,
    setFilters,
    results: searchResults.data?.results || [],
    total: searchResults.data?.total || 0,
    isLoading: searchResults.isLoading,
    error: searchResults.error,
    refetch: searchResults.refetch,
  };
}

/**
 * Search suggestions hook
 */
export function useSearchSuggestions(query: string, enabled = true) {
  const [debouncedQuery, setDebouncedQuery] = useState(query);

  // Debounce suggestions query
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedQuery(query);
    }, 200);

    return () => clearTimeout(timer);
  }, [query]);

  return useQuery({
    queryKey: searchKeys.suggestions(debouncedQuery),
    queryFn: () => searchAPI.getSuggestions(debouncedQuery),
    enabled: enabled && debouncedQuery.length >= 2,
    staleTime: 10000,
  });
}

/**
 * Recent searches hook
 */
export function useRecentSearches() {
  return useQuery({
    queryKey: searchKeys.recent(),
    queryFn: () => searchAPI.getRecentSearches(),
    staleTime: 60000, // Cache for 1 minute
  });
}

/**
 * Trending searches hook
 */
export function useTrendingSearches() {
  return useQuery({
    queryKey: searchKeys.trending(),
    queryFn: () => searchAPI.getTrendingSearches(),
    staleTime: 300000, // Cache for 5 minutes
  });
}

/**
 * Specialized search hooks for each service
 */
export function useAssetSearch(query: string, filters?: SearchFilter) {
  return useQuery({
    queryKey: [...searchKeys.all, 'assets', query, filters],
    queryFn: () => searchAPI.searchAssets(query, filters),
    enabled: query.length > 0,
    staleTime: 30000,
  });
}

export function useTransactionSearch(query: string, filters?: SearchFilter) {
  return useQuery({
    queryKey: [...searchKeys.all, 'transactions', query, filters],
    queryFn: () => searchAPI.searchTransactions(query, filters),
    enabled: query.length > 0,
    staleTime: 10000, // Shorter cache for transactions
  });
}

export function useCertificateSearch(query: string, filters?: SearchFilter) {
  return useQuery({
    queryKey: [...searchKeys.all, 'certificates', query, filters],
    queryFn: () => searchAPI.searchCertificates(query, filters),
    enabled: query.length > 0,
    staleTime: 60000,
  });
}

export function useNodeSearch(query: string, filters?: SearchFilter) {
  return useQuery({
    queryKey: [...searchKeys.all, 'nodes', query, filters],
    queryFn: () => searchAPI.searchNodes(query, filters),
    enabled: query.length > 0,
    staleTime: 5000, // Short cache for real-time node data
  });
}

/**
 * Advanced search hook with multiple data sources
 */
export function useAdvancedSearch(query: string, options?: {
  includeAssets?: boolean;
  includeTransactions?: boolean;
  includeCertificates?: boolean;
  includeNodes?: boolean;
  filters?: SearchFilter;
}) {
  const {
    includeAssets = true,
    includeTransactions = true,
    includeCertificates = true,
    includeNodes = true,
    filters,
  } = options || {};

  const assetSearch = useAssetSearch(query, { ...filters, type: ['asset'] });
  const transactionSearch = useTransactionSearch(query, { ...filters, type: ['transaction'] });
  const certificateSearch = useCertificateSearch(query, { ...filters, type: ['certificate'] });
  const nodeSearch = useNodeSearch(query, { ...filters, type: ['node'] });

  // Combine all results
  const allResults: SearchResult[] = [
    ...(includeAssets && assetSearch.data ? assetSearch.data : []),
    ...(includeTransactions && transactionSearch.data ? transactionSearch.data : []),
    ...(includeCertificates && certificateSearch.data ? certificateSearch.data : []),
    ...(includeNodes && nodeSearch.data ? nodeSearch.data : []),
  ];

  // Sort by relevance
  allResults.sort((a, b) => (b.relevance || 0) - (a.relevance || 0));

  return {
    results: allResults,
    isLoading:
      assetSearch.isLoading ||
      transactionSearch.isLoading ||
      certificateSearch.isLoading ||
      nodeSearch.isLoading,
    error:
      assetSearch.error ||
      transactionSearch.error ||
      certificateSearch.error ||
      nodeSearch.error,
    refetch: () => {
      if (includeAssets) assetSearch.refetch();
      if (includeTransactions) transactionSearch.refetch();
      if (includeCertificates) certificateSearch.refetch();
      if (includeNodes) nodeSearch.refetch();
    },
  };
}

/**
 * Live search hook with real-time updates
 */
export function useLiveSearch(query: string, updateInterval = 5000) {
  const search = useSearch(query);

  // Auto-refresh search results
  useEffect(() => {
    if (query && updateInterval > 0) {
      const interval = setInterval(() => {
        search.refetch();
      }, updateInterval);

      return () => clearInterval(interval);
    }
  }, [query, updateInterval, search.refetch]);

  return search;
}