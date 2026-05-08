// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — first-run / re-auth screen.
//
// Three modes per Phase K:
//   1. Trust Gateway     — trust.hypermesh.online relay
//   2. Private Hypermesh — yourname.hypermesh user-owned gateway
//   3. Self-hosted       — user-supplied URL
//
// User picks a mode, supplies the target URL (default for trust,
// editable for private/self-hosted), and triggers the FALCON-1024
// challenge/sign/session flow via `apiClient.connect()`.
//
// On success: navigate to Dashboard. On failure: surface the error.

import React, { useEffect, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import { apiClient } from "../api/client";
import {
  loadOrCreateFingerprint,
  previewFingerprint,
  type DeviceFingerprint,
} from "../auth/DeviceFingerprint";
import type { ConnectionMode } from "../auth/TokenStore";

const TRUST_DEFAULT_URL = "https://trust.hypermesh.online";
const PRIVATE_DEFAULT_URL = "https://yourname.hypermesh";
const SELF_HOSTED_DEFAULT_URL = "https://my-gateway.example.com:8443";

interface Props {
  onConnected: () => void;
}

export function ConnectScreen({ onConnected }: Props) {
  const [mode, setMode] = useState<ConnectionMode>("trust-gateway");
  const [baseUrl, setBaseUrl] = useState<string>(TRUST_DEFAULT_URL);
  const [fingerprint, setFingerprint] = useState<DeviceFingerprint | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadOrCreateFingerprint()
      .then(setFingerprint)
      .catch((e: unknown) => {
        setError(`device key error: ${String(e)}`);
      });
  }, []);

  function pickMode(next: ConnectionMode): void {
    setMode(next);
    setError(null);
    if (next === "trust-gateway") setBaseUrl(TRUST_DEFAULT_URL);
    else if (next === "private-domain") setBaseUrl(PRIVATE_DEFAULT_URL);
    else setBaseUrl(SELF_HOSTED_DEFAULT_URL);
  }

  async function handleConnect(): Promise<void> {
    if (!baseUrl.startsWith("http")) {
      setError("URL must start with https://");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await apiClient.connect({ baseUrl, mode });
      onConnected();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      Alert.alert("Connection failed", msg);
    } finally {
      setBusy(false);
    }
  }

  return (
    <ScrollView className="flex-1 bg-bg" contentContainerStyle={{ padding: 24 }}>
      <Text className="text-text text-3xl font-semibold mb-2">HyperMesh</Text>
      <Text className="text-muted text-sm mb-6">
        Phone is a remote client. Pick the daemon you trust.
      </Text>

      <View className="mb-6">
        <Text className="text-muted text-xs uppercase tracking-wider mb-2">
          Mode
        </Text>
        <ModeButton
          label="Trust Gateway"
          subtitle="trust.hypermesh.online — foundation relay"
          selected={mode === "trust-gateway"}
          onPress={() => pickMode("trust-gateway")}
        />
        <ModeButton
          label="Private Hypermesh"
          subtitle="yourname.hypermesh — your foundation grant"
          selected={mode === "private-domain"}
          onPress={() => pickMode("private-domain")}
        />
        <ModeButton
          label="Self-hosted"
          subtitle="your own gateway, your own TLS"
          selected={mode === "self-hosted"}
          onPress={() => pickMode("self-hosted")}
        />
      </View>

      <View className="mb-6">
        <Text className="text-muted text-xs uppercase tracking-wider mb-2">
          Gateway URL
        </Text>
        <TextInput
          value={baseUrl}
          onChangeText={setBaseUrl}
          autoCapitalize="none"
          autoCorrect={false}
          keyboardType="url"
          editable={!busy}
          className="bg-surface border border-border rounded-lg px-4 py-3 text-text"
          placeholder="https://..."
          placeholderTextColor="#71717a"
        />
      </View>

      <View className="mb-6">
        <Text className="text-muted text-xs uppercase tracking-wider mb-2">
          Device Identity
        </Text>
        <View className="bg-surface border border-border rounded-lg px-4 py-3">
          <Text className="text-text font-mono text-sm">
            {fingerprint
              ? previewFingerprint(fingerprint)
              : "generating…"}
          </Text>
          {fingerprint && (
            <Text className="text-muted text-xs mt-1">
              {fingerprint.algorithm}
              {fingerprint.algorithm === "ecdsa-p256-placeholder" && (
                <Text className="text-accent"> (alpha — UniFFI lands C.4.5)</Text>
              )}
            </Text>
          )}
        </View>
      </View>

      {error && (
        <View className="bg-red-950 border border-red-800 rounded-lg px-4 py-3 mb-4">
          <Text className="text-red-200 text-sm">{error}</Text>
        </View>
      )}

      <Pressable
        accessibilityRole="button"
        disabled={busy || !fingerprint}
        onPress={handleConnect}
        className={`rounded-lg px-4 py-4 items-center ${
          busy || !fingerprint ? "bg-elevated" : "bg-accent"
        }`}
      >
        {busy ? (
          <ActivityIndicator color="#fafafa" />
        ) : (
          <Text className="text-text font-semibold">Connect</Text>
        )}
      </Pressable>

      <Text className="text-muted text-xs mt-6 leading-5">
        On Connect, your device public key is sent to the gateway. The gateway
        challenges you to sign random bytes; we sign with the key stored in
        your phone's secure enclave (Keychain/Keystore). The daemon issues a
        capability token bound to this device. The token is the only credential
        used for subsequent calls.
      </Text>
    </ScrollView>
  );
}

interface ModeButtonProps {
  label: string;
  subtitle: string;
  selected: boolean;
  onPress: () => void;
}

function ModeButton({ label, subtitle, selected, onPress }: ModeButtonProps) {
  return (
    <Pressable
      onPress={onPress}
      accessibilityRole="button"
      accessibilityState={{ selected }}
      className={`rounded-lg px-4 py-3 mb-2 border ${
        selected ? "border-accent bg-elevated" : "border-border bg-surface"
      }`}
    >
      <Text className="text-text font-medium">{label}</Text>
      <Text className="text-muted text-xs mt-1">{subtitle}</Text>
    </Pressable>
  );
}
