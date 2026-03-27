// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  ArrowRight,
  Minimize2,
  Lock,
  Grid3X3,
  Send,
  Terminal,
  Info,
} from 'lucide-react';

interface PipelineStage {
  name: string;
  description: string;
  detail: string;
  icon: React.ElementType;
  color: string;
}

const STAGES: PipelineStage[] = [
  {
    name: 'Compress',
    description: 'Zstd/Brotli compression',
    detail: 'Content-type auto-detection: text uses Brotli, binary uses Zstd, video/audio skipped. Per-segment compression.',
    icon: Minimize2,
    color: 'text-blue-400',
  },
  {
    name: 'Encrypt',
    description: 'Kyber-1024 KEM + AES-256-GCM',
    detail: 'One Kyber-1024 KEM shared secret, per-segment keys derived via BLAKE3-HKDF. Post-quantum security (R7).',
    icon: Lock,
    color: 'text-green-400',
  },
  {
    name: 'Shard',
    description: 'Reed-Solomon erasure coding',
    detail: 'Default 10-of-14 RS parameters. k+m configurable. Adaptive sizing based on asset size (R14). Shard is the atomic content-addressed unit.',
    icon: Grid3X3,
    color: 'text-yellow-400',
  },
  {
    name: 'Distribute',
    description: 'Tensor-based matrix placement',
    detail: 'Shards placed at calculated matrix positions using tensor operations. Geographic and network proximity considered.',
    icon: Send,
    color: 'text-purple-400',
  },
];

export function StorePipeline() {
  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-white">Store Pipeline</h2>

      {/* Pipeline diagram */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Info className="h-5 w-5 text-cyan-400" />
            Data Processing Pipeline
          </CardTitle>
          <CardDescription className="text-gray-400">
            The exact order data is processed: Compress, Encrypt, Shard, Distribute (R3)
          </CardDescription>
        </CardHeader>
        <CardContent>
          {/* Visual pipeline */}
          <div className="flex flex-col md:flex-row items-center justify-center gap-2 md:gap-4 mb-8">
            {STAGES.map((stage, idx) => {
              const Icon = stage.icon;
              return (
                <React.Fragment key={stage.name}>
                  <div className="flex flex-col items-center p-4 rounded-lg border border-cyan-500/20 bg-cyan-500/5 min-w-[120px]">
                    <Icon className={`h-6 w-6 ${stage.color} mb-2`} />
                    <span className="text-white font-medium text-sm">{stage.name}</span>
                    <span className="text-xs text-gray-400 text-center mt-1">
                      {stage.description}
                    </span>
                  </div>
                  {idx < STAGES.length - 1 && (
                    <ArrowRight className="h-5 w-5 text-cyan-400 shrink-0 rotate-90 md:rotate-0" />
                  )}
                </React.Fragment>
              );
            })}
          </div>

          {/* Stage details */}
          <div className="space-y-3">
            {STAGES.map((stage, idx) => {
              const Icon = stage.icon;
              return (
                <div
                  key={stage.name}
                  className="p-3 rounded-lg border border-cyan-500/10 bg-black/20"
                >
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="outline" className="text-xs bg-cyan-500/10 text-cyan-400 border-cyan-500/30">
                      Step {idx + 1}
                    </Badge>
                    <Icon className={`h-4 w-4 ${stage.color}`} />
                    <span className="text-white font-medium text-sm">{stage.name}</span>
                  </div>
                  <p className="text-sm text-gray-400 ml-6">{stage.detail}</p>
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* CLI usage */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Terminal className="h-5 w-5 text-cyan-400" />
            CLI Usage
          </CardTitle>
          <CardDescription className="text-gray-400">
            Store and retrieve content via the HyperMesh CLI
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <p className="text-sm text-gray-400">Store a file:</p>
            <div className="p-3 rounded-lg bg-black/60 border border-cyan-500/20 font-mono text-sm text-cyan-400">
              hypermesh store &lt;file-path&gt;
            </div>
          </div>
          <div className="space-y-2">
            <p className="text-sm text-gray-400">Retrieve by asset ID:</p>
            <div className="p-3 rounded-lg bg-black/60 border border-cyan-500/20 font-mono text-sm text-cyan-400">
              hypermesh retrieve &lt;asset-id&gt; --output &lt;path&gt;
            </div>
          </div>
          <div className="space-y-2">
            <p className="text-sm text-gray-400">Validate the blockchain:</p>
            <div className="p-3 rounded-lg bg-black/60 border border-cyan-500/20 font-mono text-sm text-cyan-400">
              hypermesh blockchain validate
            </div>
          </div>
          <p className="text-xs text-gray-500">
            The UI store pipeline will be available once the IPC daemon exposes
            streaming store/retrieve endpoints. For now, use the CLI above.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
