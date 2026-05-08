// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — Assets tab.
//
// Read-only views (ViewOnly capability sufficient):
//   - `caesar.balance` — gold-grams balance + tier
//   - `asset.list`     — assets owned/visible to this device

import React from "react";
import {
  ActivityIndicator,
  RefreshControl,
  ScrollView,
  Text,
  View,
} from "react-native";
import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../api/client";
import type { CaesarBalance, AssetListResponse } from "@hypermesh/sdk";

export function AssetsScreen() {
  const balance = useQuery<CaesarBalance>({
    queryKey: ["caesar.balance"],
    queryFn: () => apiClient.invoke((c) => c.caesar.balance()),
    refetchInterval: 60_000,
  });

  const assets = useQuery<AssetListResponse>({
    queryKey: ["asset.list"],
    queryFn: () => apiClient.invoke((c) => c.asset.list()),
    refetchInterval: 60_000,
  });

  const refreshing = balance.isFetching || assets.isFetching;
  function onRefresh(): void {
    balance.refetch();
    assets.refetch();
  }

  return (
    <ScrollView
      className="flex-1 bg-bg"
      contentContainerStyle={{ padding: 24 }}
      refreshControl={
        <RefreshControl
          refreshing={refreshing}
          onRefresh={onRefresh}
          tintColor="#fafafa"
        />
      }
    >
      <Text className="text-text text-2xl font-semibold mb-6">Assets</Text>

      <View className="bg-surface border border-border rounded-lg px-4 py-4 mb-4">
        <Text className="text-muted text-xs uppercase tracking-wider mb-2">
          Caesar balance
        </Text>
        {balance.isLoading ? (
          <ActivityIndicator color="#fafafa" />
        ) : balance.error ? (
          <Text className="text-red-400 text-sm">{String(balance.error)}</Text>
        ) : balance.data ? (
          <View>
            <Text className="text-text text-3xl font-mono">
              {balance.data.gold_grams}
            </Text>
            <Text className="text-muted text-xs mt-1">
              gold-grams · ~${balance.data.usd_equivalent} USD
            </Text>
            <Text className="text-muted text-xs mt-2">
              tier: {balance.data.tier}
            </Text>
          </View>
        ) : null}
      </View>

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Owned assets
      </Text>
      {assets.isLoading ? (
        <ActivityIndicator color="#fafafa" />
      ) : assets.error ? (
        <Text className="text-red-400 text-sm">{String(assets.error)}</Text>
      ) : (
        (assets.data?.assets ?? []).map((a, idx: number) => (
          <View
            key={`${a.content_hash}-${idx}`}
            className="bg-surface border border-border rounded-lg px-4 py-3 mb-2"
          >
            <Text className="text-text font-mono text-sm" numberOfLines={1}>
              {a.content_hash}
            </Text>
            <View className="flex-row justify-between mt-1">
              <Text className="text-muted text-xs">
                {a.category} · {a.scope}
              </Text>
              <Text className="text-muted text-xs">
                block #{a.block_index}
              </Text>
            </View>
          </View>
        ))
      )}
    </ScrollView>
  );
}
