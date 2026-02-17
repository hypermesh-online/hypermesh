// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Shield, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SecuritySettings {
  quantumSafe: boolean;
}

interface SecurityModeSelectorProps {
  settings: SecuritySettings;
  onSettingsChange: (updates: Partial<SecuritySettings>) => void;
}

export function SecurityModeSelector({
  settings,
  onSettingsChange
}: SecurityModeSelectorProps) {
  return (
    <Card className="border-quantum-200 bg-gradient-to-r from-quantum-50 to-purple-50">
      <CardHeader>
        <CardTitle className="text-lg">Security Mode Selection</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-4">
          <div 
            className={cn(
              "p-4 border-2 rounded-lg cursor-pointer transition-colors",
              settings.quantumSafe 
                ? "border-quantum-500 bg-quantum-50" 
                : "border-gray-200 hover:border-quantum-300"
            )}
            onClick={() => onSettingsChange({ quantumSafe: true })}
          >
            <div className="flex items-center space-x-3">
              <div className={cn(
                "w-4 h-4 rounded-full border-2",
                settings.quantumSafe 
                  ? "border-quantum-500 bg-quantum-500" 
                  : "border-gray-300"
              )}>
                {settings.quantumSafe && (
                  <div className="w-2 h-2 bg-white rounded-full m-0.5"></div>
                )}
              </div>
              <div className="flex items-center space-x-2">
                <Shield className="h-5 w-5 text-quantum-600" />
                <span className="font-semibold">Quantum-Safe Mode</span>
                <Badge variant="outline" className="text-green-600 border-green-600">
                  Recommended
                </Badge>
              </div>
            </div>
            <div className="ml-7 mt-2 text-sm text-muted-foreground">
              <ul className="space-y-1">
                <li>• FALCON-1024 post-quantum digital signatures</li>
                <li>• Kyber key exchange mechanism</li>
                <li>• Future-proof against quantum computer attacks</li>
                <li>• NIST-standardized cryptographic algorithms</li>
              </ul>
            </div>
          </div>

          <div 
            className={cn(
              "p-4 border-2 rounded-lg cursor-pointer transition-colors",
              !settings.quantumSafe 
                ? "border-orange-500 bg-orange-50" 
                : "border-gray-200 hover:border-orange-300"
            )}
            onClick={() => onSettingsChange({ quantumSafe: false })}
          >
            <div className="flex items-center space-x-3">
              <div className={cn(
                "w-4 h-4 rounded-full border-2",
                !settings.quantumSafe 
                  ? "border-orange-500 bg-orange-500" 
                  : "border-gray-300"
              )}>
                {!settings.quantumSafe && (
                  <div className="w-2 h-2 bg-white rounded-full m-0.5"></div>
                )}
              </div>
              <div className="flex items-center space-x-2">
                <AlertTriangle className="h-5 w-5 text-orange-600" />
                <span className="font-semibold">Traditional Mode</span>
                <Badge variant="outline" className="text-orange-600 border-orange-600">
                  Not Recommended
                </Badge>
              </div>
            </div>
            <div className="ml-7 mt-2 text-sm text-muted-foreground">
              <ul className="space-y-1">
                <li>• RSA/ECDSA signatures (quantum-vulnerable)</li>
                <li>• ECDH key exchange (quantum-vulnerable)</li>
                <li>• Legacy certificate validation</li>
                <li>• Will become insecure with quantum computers</li>
              </ul>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}