// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Shield, Zap } from 'lucide-react';

export function ProtocolDetails() {
  return (
    <div className="space-y-6">
      <div className="text-center py-4">
        <h2 className="text-2xl font-bold text-white mb-2">STOQ Protocol Architecture</h2>
        <p className="text-gray-400">Secure Tokenization Over QUIC implementation details</p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Shield className="h-5 w-5 text-cyan-400" />
              Protocol Stack
            </CardTitle>
            <CardDescription className="text-gray-400">STOQ protocol layer implementation</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {[
                { layer: 'Application Layer', description: 'HyperMesh dApps', status: 'active' },
                { layer: 'STOQ Tokenization', description: 'Secure token management', status: 'active' },
                { layer: 'QUIC Transport', description: 'HTTP/3 with multiplexing', status: 'active' },
                { layer: 'TLS 1.3 Security', description: 'End-to-end encryption', status: 'active' },
                { layer: 'IPv6 Network', description: 'Direct P2P addressing', status: 'active' },
                { layer: 'Physical Layer', description: 'Global infrastructure', status: 'active' },
              ].reverse().map((layer, i) => (
                <div key={i} className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                  <div className="flex justify-between items-center">
                    <div>
                      <h4 className="font-medium text-white">{layer.layer}</h4>
                      <p className="text-sm text-gray-400">{layer.description}</p>
                    </div>
                    <Badge variant="default" className="bg-green-500/20 text-green-400 border-green-500/30">
                      {layer.status}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Zap className="h-5 w-5 text-cyan-400" />
              Performance Features
            </CardTitle>
            <CardDescription className="text-gray-400">What makes STOQ faster than traditional protocols</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {[
                {
                  feature: 'Zero-RTT Connection',
                  improvement: '50% faster',
                  description: 'Resume connections without handshake'
                },
                {
                  feature: 'Stream Multiplexing',
                  improvement: '3x efficiency',
                  description: 'Multiple streams over single connection'
                },
                {
                  feature: 'Loss Recovery',
                  improvement: '90% faster',
                  description: 'Per-stream packet loss recovery'
                },
                {
                  feature: 'Header Compression',
                  improvement: '60% reduction',
                  description: 'QPACK compression algorithm'
                },
                {
                  feature: 'Connection Migration',
                  improvement: 'Seamless',
                  description: 'Survive network changes'
                }
              ].map((item, i) => (
                <div key={i} className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
                  <div className="flex justify-between items-start mb-2">
                    <h4 className="font-medium text-white">{item.feature}</h4>
                    <Badge variant="outline" className="border-cyan-500/50 text-cyan-400">
                      {item.improvement}
                    </Badge>
                  </div>
                  <p className="text-sm text-gray-400">{item.description}</p>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white">Tokenization Security</CardTitle>
          <CardDescription className="text-gray-400">How STOQ secures data transmission with cryptographic tokens</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-6 lg:grid-cols-3">
            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-3">Token Generation</h4>
              <div className="space-y-2 text-sm text-gray-300">
                <div>• FALCON-1024 signatures</div>
                <div>• Quantum-resistant security</div>
                <div>• Per-session token rotation</div>
                <div>• Zero-knowledge proofs</div>
              </div>
            </div>

            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-3">Token Validation</h4>
              <div className="space-y-2 text-sm text-gray-300">
                <div>• Distributed verification</div>
                <div>• Byzantine fault tolerance</div>
                <div>• Real-time validation</div>
                <div>• Automatic token refresh</div>
              </div>
            </div>

            <div className="p-4 rounded-lg bg-cyan-500/10 border border-cyan-500/20">
              <h4 className="font-medium text-white mb-3">Security Benefits</h4>
              <div className="space-y-2 text-sm text-gray-300">
                <div>• No bearer token vulnerabilities</div>
                <div>• Forward/backward secrecy</div>
                <div>• Replay attack prevention</div>
                <div>• Identity privacy protection</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}