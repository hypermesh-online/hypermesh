// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — Dashboard tab.
//
// Polls `node.status` every 30s. Shows:
//   - daemon status string (running/syncing/error)
//   - peer count
//   - device fingerprint (truncated)
//   - active gateway base URL + mode
//   - sign-out button

import React from "react";
import {
  ActivityIndicator,
  Pressable,
  RefreshControl,
  ScrollView,
  Text,
  View,
} from "react-native";
import { useQuery } from "@tanstack/react-query";
import { apiClient } from "../api/client";
import { previewFingerprint } from "../auth/DeviceFingerprint";
import type { NodeStatus, PeersResponse } from "@hypermesh/sdk";

interface Props {
  onSignedOut: () => void;
}

export function DashboardScreen({ onSignedOut }: Props) {
  const session = apiClient.currentSession();
  const fingerprint = apiClient.currentFingerprint();

  const statusQuery = useQuery<NodeStatus>({
    queryKey: ["node.status"],
    queryFn: () => apiClient.invoke((c) => c.node.status()),
    refetchInterval: 30_000,
  });

  const peersQuery = useQuery<PeersResponse>({
    queryKey: ["network.peers"],
    queryFn: () => apiClient.invoke((c) => c.network.peers()),
    refetchInterval: 30_000,
  });

  async function handleSignOut(): Promise<void> {
    await apiClient.signOutRemote();
    onSignedOut();
  }

  const refreshing = statusQuery.isFetching || peersQuery.isFetching;
  function onRefresh(): void {
    statusQuery.refetch();
    peersQuery.refetch();
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
      <Text className="text-text text-2xl font-semibold mb-6">Dashboard</Text>

      <Card title="Daemon">
        {statusQuery.isLoading ? (
          <ActivityIndicator color="#fafafa" />
        ) : statusQuery.error ? (
          <Text className="text-red-400 text-sm">
            {String(statusQuery.error)}
          </Text>
        ) : statusQuery.data ? (
          <View>
            <KV k="node" v={statusQuery.data.node_id} />
            <KV k="height" v={String(statusQuery.data.chain_height)} />
            <KV k="privacy" v={statusQuery.data.privacy_mode} />
            <KV k="uptime" v={`${statusQuery.data.uptime_secs}s`} />
          </View>
        ) : null}
      </Card>

      <Card title="Peers">
        {peersQuery.isLoading ? (
          <ActivityIndicator color="#fafafa" />
        ) : peersQuery.error ? (
          <Text className="text-red-400 text-sm">
            {String(peersQuery.error)}
          </Text>
        ) : (
          <KV
            k="connected"
            v={String(peersQuery.data?.peers?.length ?? 0)}
          />
        )}
      </Card>

      <Card title="This device">
        <KV
          k="fingerprint"
          v={fingerprint ? previewFingerprint(fingerprint) : "?"}
        />
        <KV k="alg" v={fingerprint?.algorithm ?? "?"} />
      </Card>

      <Card title="Connected to">
        <KV k="mode" v={session?.mode ?? "?"} />
        <KV k="url" v={session?.baseUrl ?? "?"} />
        <KV
          k="expires"
          v={
            session
              ? new Date(session.expiresAtSecs * 1000).toLocaleString()
              : "?"
          }
        />
      </Card>

      <Pressable
        onPress={handleSignOut}
        accessibilityRole="button"
        className="bg-red-900 rounded-lg px-4 py-3 items-center mt-4"
      >
        <Text className="text-text font-medium">Sign out</Text>
      </Pressable>
    </ScrollView>
  );
}

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <View className="bg-surface border border-border rounded-lg px-4 py-3 mb-3">
      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        {title}
      </Text>
      {children}
    </View>
  );
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <View className="flex-row justify-between py-1">
      <Text className="text-muted text-sm">{k}</Text>
      <Text className="text-text text-sm font-mono" numberOfLines={1}>
        {v}
      </Text>
    </View>
  );
}
