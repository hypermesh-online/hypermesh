// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — DNS tab.
//
// `dns.resolve` lookup + listing of own DNS records. Read-only for
// ViewOnly capability; future register flows will require AssetWrite.

import React, { useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import { useQuery, useMutation } from "@tanstack/react-query";
import { apiClient } from "../api/client";
import type { DnsListResponse, DnsResolveResponse } from "@hypermesh/sdk";

export function DnsScreen() {
  const [query, setQuery] = useState("");
  const [resolved, setResolved] = useState<DnsResolveResponse | null>(null);

  const own = useQuery<DnsListResponse>({
    queryKey: ["dns.list"],
    queryFn: () => apiClient.invoke((c) => c.dns.list()),
    refetchInterval: 60_000,
  });

  const resolveMut = useMutation({
    mutationFn: async (name: string) => {
      return apiClient.invoke((c) => c.dns.resolve(name));
    },
    onSuccess: setResolved,
    onError: (e) => Alert.alert("Resolve failed", String(e)),
  });

  return (
    <ScrollView
      className="flex-1 bg-bg"
      contentContainerStyle={{ padding: 24 }}
    >
      <Text className="text-text text-2xl font-semibold mb-6">DNS</Text>

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Resolve
      </Text>
      <TextInput
        value={query}
        onChangeText={setQuery}
        autoCapitalize="none"
        autoCorrect={false}
        keyboardType="url"
        placeholder="something.hypermesh"
        placeholderTextColor="#71717a"
        className="bg-surface border border-border rounded-lg px-4 py-3 text-text mb-2"
      />
      <Pressable
        onPress={() => query && resolveMut.mutate(query)}
        disabled={!query || resolveMut.isPending}
        accessibilityRole="button"
        className={`rounded-lg px-4 py-3 items-center mb-2 ${
          !query || resolveMut.isPending ? "bg-elevated" : "bg-accent"
        }`}
      >
        {resolveMut.isPending ? (
          <ActivityIndicator color="#fafafa" />
        ) : (
          <Text className="text-text font-medium">Resolve</Text>
        )}
      </Pressable>

      {resolved && (
        <View className="bg-surface border border-border rounded-lg px-4 py-3 mb-6">
          <Text className="text-muted text-xs uppercase tracking-wider mb-2">
            Result
          </Text>
          <Text className="text-text font-mono text-xs" selectable>
            {JSON.stringify(resolved, null, 2)}
          </Text>
        </View>
      )}

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Your records
      </Text>
      {own.isLoading ? (
        <ActivityIndicator color="#fafafa" />
      ) : own.error ? (
        <Text className="text-red-400 text-sm">{String(own.error)}</Text>
      ) : (own.data?.records ?? []).length === 0 ? (
        <Text className="text-muted text-sm">No records.</Text>
      ) : (
        (own.data?.records ?? []).map((r, idx) => (
          <View
            key={`${r.name}-${idx}`}
            className="bg-surface border border-border rounded-lg px-4 py-3 mb-2"
          >
            <Text className="text-text text-sm font-mono" numberOfLines={1}>
              {r.name}
            </Text>
            <Text className="text-muted text-xs mt-1">
              → {r.address}
            </Text>
          </View>
        ))
      )}
    </ScrollView>
  );
}
