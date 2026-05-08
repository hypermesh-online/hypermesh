// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — Share tab.
//
// Read-side: `share.inbox` shows pending shares.
// Write-side: `share.send` — uses Expo's native document picker to
// pick a file, then forwards the picked URI to the daemon. The daemon
// is responsible for actually reading and sharding the file (the phone
// is a remote client; it has no local node).
//
// The daemon-side IPC `share.send` is not yet enumerated in the K.2
// SDK; the call is wired here as a typed JSON-RPC POST so the screen
// is functional the moment the route lands.

import React, { useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Pressable,
  RefreshControl,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import * as DocumentPicker from "expo-document-picker";
import { useQuery, useMutation } from "@tanstack/react-query";
import { apiClient } from "../api/client";

interface ShareInboxEntry {
  id?: string;
  from?: string;
  filename?: string;
  size_bytes?: number;
  received_at?: string;
}

interface ShareInboxResponse {
  shares?: ShareInboxEntry[];
}

export function ShareScreen() {
  const [recipient, setRecipient] = useState("");
  const [pickedUri, setPickedUri] = useState<string | null>(null);
  const [pickedName, setPickedName] = useState<string | null>(null);

  const inbox = useQuery<ShareInboxResponse>({
    queryKey: ["share.inbox"],
    queryFn: async () => {
      const client = apiClient.client();
      if (!client) throw new Error("not connected");
      const session = apiClient.currentSession();
      if (!session) throw new Error("no session");
      // Direct fetch — `share.inbox` IPC not yet in TS SDK.
      const r = await fetch(`${session.baseUrl}/api/v1/share/inbox`, {
        headers: {
          Accept: "application/json",
          "X-HyperMesh-Capability": session.token,
        },
      });
      if (!r.ok) {
        if (r.status === 404) return { shares: [] };
        throw new Error(`HTTP ${r.status}`);
      }
      return r.json() as Promise<ShareInboxResponse>;
    },
    refetchInterval: 30_000,
  });

  const sendMut = useMutation({
    mutationFn: async (args: {
      recipient: string;
      uri: string;
      filename: string;
    }) => {
      const session = apiClient.currentSession();
      if (!session) throw new Error("no session");
      const r = await fetch(`${session.baseUrl}/api/v1/share/send`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "X-HyperMesh-Capability": session.token,
        },
        body: JSON.stringify({
          recipient: args.recipient,
          source_uri: args.uri,
          filename: args.filename,
        }),
      });
      if (!r.ok) {
        const body = await r.text().catch(() => "");
        throw new Error(`HTTP ${r.status}: ${body}`);
      }
      return r.json();
    },
    onSuccess: () => {
      Alert.alert("Sent", "share.send accepted");
      setPickedUri(null);
      setPickedName(null);
      setRecipient("");
      inbox.refetch();
    },
    onError: (e) => {
      Alert.alert("Send failed", String(e));
    },
  });

  async function pickFile(): Promise<void> {
    const r = await DocumentPicker.getDocumentAsync({
      copyToCacheDirectory: false,
      multiple: false,
    });
    if (r.canceled) return;
    const asset = r.assets?.[0];
    if (!asset) return;
    setPickedUri(asset.uri);
    setPickedName(asset.name);
  }

  return (
    <ScrollView
      className="flex-1 bg-bg"
      contentContainerStyle={{ padding: 24 }}
      refreshControl={
        <RefreshControl
          refreshing={inbox.isFetching}
          onRefresh={() => inbox.refetch()}
          tintColor="#fafafa"
        />
      }
    >
      <Text className="text-text text-2xl font-semibold mb-6">Share</Text>

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Send
      </Text>
      <TextInput
        value={recipient}
        onChangeText={setRecipient}
        autoCapitalize="none"
        autoCorrect={false}
        placeholder="recipient identity (hex pubkey or domain)"
        placeholderTextColor="#71717a"
        className="bg-surface border border-border rounded-lg px-4 py-3 text-text mb-2"
      />
      <Pressable
        onPress={pickFile}
        accessibilityRole="button"
        className="bg-surface border border-border rounded-lg px-4 py-3 mb-2"
      >
        <Text className="text-text">
          {pickedName ? `Picked: ${pickedName}` : "Pick file…"}
        </Text>
      </Pressable>
      <Pressable
        onPress={() =>
          pickedUri &&
          recipient &&
          pickedName &&
          sendMut.mutate({
            recipient,
            uri: pickedUri,
            filename: pickedName,
          })
        }
        disabled={!pickedUri || !recipient || sendMut.isPending}
        accessibilityRole="button"
        className={`rounded-lg px-4 py-3 items-center mb-6 ${
          !pickedUri || !recipient || sendMut.isPending
            ? "bg-elevated"
            : "bg-accent"
        }`}
      >
        {sendMut.isPending ? (
          <ActivityIndicator color="#fafafa" />
        ) : (
          <Text className="text-text font-medium">Send</Text>
        )}
      </Pressable>

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Inbox
      </Text>
      {inbox.isLoading ? (
        <ActivityIndicator color="#fafafa" />
      ) : inbox.error ? (
        <Text className="text-red-400 text-sm">{String(inbox.error)}</Text>
      ) : (inbox.data?.shares ?? []).length === 0 ? (
        <Text className="text-muted text-sm">No pending shares.</Text>
      ) : (
        (inbox.data?.shares ?? []).map((s, idx) => (
          <View
            key={s.id ?? idx}
            className="bg-surface border border-border rounded-lg px-4 py-3 mb-2"
          >
            <Text className="text-text text-sm" numberOfLines={1}>
              {s.filename ?? "(unnamed)"}
            </Text>
            <Text className="text-muted text-xs mt-1" numberOfLines={1}>
              from {s.from ?? "?"}
            </Text>
            <Text className="text-muted text-xs">
              {s.size_bytes != null ? `${s.size_bytes} B` : ""} ·{" "}
              {s.received_at ?? ""}
            </Text>
          </View>
        ))
      )}
    </ScrollView>
  );
}
