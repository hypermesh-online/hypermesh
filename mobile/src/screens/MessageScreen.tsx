// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — Message tab.
//
// `message.inbox` + `message.send`. Wallet/AssetWrite capability
// required for send. The IPC routes are not yet enumerated in the K.2
// TS SDK; we POST raw JSON-RPC bodies directly.

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
import { useQuery, useMutation } from "@tanstack/react-query";
import { apiClient } from "../api/client";

interface MessageEntry {
  id?: string;
  from?: string;
  body?: string;
  received_at?: string;
}

interface MessageInboxResponse {
  messages?: MessageEntry[];
}

export function MessageScreen() {
  const [to, setTo] = useState("");
  const [body, setBody] = useState("");

  const inbox = useQuery<MessageInboxResponse>({
    queryKey: ["message.inbox"],
    queryFn: async () => {
      const session = apiClient.currentSession();
      if (!session) throw new Error("no session");
      const r = await fetch(`${session.baseUrl}/api/v1/message/inbox`, {
        headers: {
          Accept: "application/json",
          "X-HyperMesh-Capability": session.token,
        },
      });
      if (!r.ok) {
        if (r.status === 404) return { messages: [] };
        throw new Error(`HTTP ${r.status}`);
      }
      return r.json() as Promise<MessageInboxResponse>;
    },
    refetchInterval: 30_000,
  });

  const sendMut = useMutation({
    mutationFn: async (args: { to: string; body: string }) => {
      const session = apiClient.currentSession();
      if (!session) throw new Error("no session");
      const r = await fetch(`${session.baseUrl}/api/v1/message/send`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "X-HyperMesh-Capability": session.token,
        },
        body: JSON.stringify({ to: args.to, body: args.body }),
      });
      if (!r.ok) {
        const txt = await r.text().catch(() => "");
        throw new Error(`HTTP ${r.status}: ${txt}`);
      }
      return r.json();
    },
    onSuccess: () => {
      Alert.alert("Sent", "message.send accepted");
      setTo("");
      setBody("");
      inbox.refetch();
    },
    onError: (e) => Alert.alert("Send failed", String(e)),
  });

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
      <Text className="text-text text-2xl font-semibold mb-6">Messages</Text>

      <Text className="text-muted text-xs uppercase tracking-wider mb-2">
        Compose
      </Text>
      <TextInput
        value={to}
        onChangeText={setTo}
        autoCapitalize="none"
        autoCorrect={false}
        placeholder="to (hex pubkey or domain)"
        placeholderTextColor="#71717a"
        className="bg-surface border border-border rounded-lg px-4 py-3 text-text mb-2"
      />
      <TextInput
        value={body}
        onChangeText={setBody}
        multiline
        numberOfLines={4}
        placeholder="message body"
        placeholderTextColor="#71717a"
        className="bg-surface border border-border rounded-lg px-4 py-3 text-text mb-2 min-h-24"
      />
      <Pressable
        onPress={() =>
          to && body && sendMut.mutate({ to, body })
        }
        disabled={!to || !body || sendMut.isPending}
        accessibilityRole="button"
        className={`rounded-lg px-4 py-3 items-center mb-6 ${
          !to || !body || sendMut.isPending ? "bg-elevated" : "bg-accent"
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
      ) : (inbox.data?.messages ?? []).length === 0 ? (
        <Text className="text-muted text-sm">No messages.</Text>
      ) : (
        (inbox.data?.messages ?? []).map((m, idx) => (
          <View
            key={m.id ?? idx}
            className="bg-surface border border-border rounded-lg px-4 py-3 mb-2"
          >
            <Text className="text-muted text-xs" numberOfLines={1}>
              from {m.from ?? "?"} · {m.received_at ?? ""}
            </Text>
            <Text className="text-text text-sm mt-2">{m.body ?? ""}</Text>
          </View>
        ))
      )}
    </ScrollView>
  );
}
