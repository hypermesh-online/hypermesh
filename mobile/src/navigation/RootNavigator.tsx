// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.4 — root navigator.
//
// Two states:
//   - "connect"   → ConnectScreen (no session, or session expired)
//   - "connected" → bottom-tab navigator: Dashboard / Assets / Share /
//                   Messages / DNS
//
// Re-auth: `apiClient.onReauthRequired` flips state back to "connect"
// when the daemon emits -32004.

import React, { useCallback, useEffect, useState } from "react";
import { ActivityIndicator, Text, View } from "react-native";
import { NavigationContainer, DarkTheme } from "@react-navigation/native";
import { createBottomTabNavigator } from "@react-navigation/bottom-tabs";
import { apiClient } from "../api/client";
import { ConnectScreen } from "../screens/ConnectScreen";
import { DashboardScreen } from "../screens/DashboardScreen";
import { AssetsScreen } from "../screens/AssetsScreen";
import { ShareScreen } from "../screens/ShareScreen";
import { MessageScreen } from "../screens/MessageScreen";
import { DnsScreen } from "../screens/DnsScreen";

type RootState = "loading" | "connect" | "connected";

const Tab = createBottomTabNavigator();

const navTheme = {
  ...DarkTheme,
  colors: {
    ...DarkTheme.colors,
    background: "#0a0a0a",
    card: "#161616",
    text: "#fafafa",
    border: "#2a2a2a",
    primary: "#f97316",
    notification: "#f97316",
  },
};

export function RootNavigator() {
  const [state, setState] = useState<RootState>("loading");

  // Bootstrap on mount.
  useEffect(() => {
    let cancelled = false;
    apiClient
      .bootstrap()
      .then((ok) => {
        if (cancelled) return;
        setState(ok ? "connected" : "connect");
      })
      .catch(() => {
        if (cancelled) return;
        setState("connect");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // React to capability-denial mid-flight.
  useEffect(() => {
    const unsub = apiClient.onReauthRequired(() => {
      setState("connect");
    });
    return unsub;
  }, []);

  const handleConnected = useCallback(() => setState("connected"), []);
  const handleSignedOut = useCallback(() => setState("connect"), []);

  if (state === "loading") {
    return (
      <View className="flex-1 items-center justify-center bg-bg">
        <ActivityIndicator color="#fafafa" />
        <Text className="text-muted text-xs mt-2">starting…</Text>
      </View>
    );
  }

  if (state === "connect") {
    return <ConnectScreen onConnected={handleConnected} />;
  }

  return (
    <NavigationContainer theme={navTheme}>
      <Tab.Navigator
        screenOptions={{
          headerShown: false,
          tabBarStyle: { backgroundColor: "#161616", borderTopColor: "#2a2a2a" },
          tabBarActiveTintColor: "#f97316",
          tabBarInactiveTintColor: "#71717a",
        }}
      >
        <Tab.Screen name="Home">
          {() => <DashboardScreen onSignedOut={handleSignedOut} />}
        </Tab.Screen>
        <Tab.Screen name="Assets" component={AssetsScreen} />
        <Tab.Screen name="Share" component={ShareScreen} />
        <Tab.Screen name="Messages" component={MessageScreen} />
        <Tab.Screen name="DNS" component={DnsScreen} />
      </Tab.Navigator>
    </NavigationContainer>
  );
}
