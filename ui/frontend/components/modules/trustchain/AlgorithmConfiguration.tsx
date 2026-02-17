// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { 
  Key, 
  Lock,
  Info, 
  ChevronDown, 
  ChevronRight, 
  CheckCircle,
  Zap,
  ExternalLink
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface SecuritySettings {
  quantumSafe: boolean;
  falconSigning: boolean;
  kyberKeyExchange: boolean;
}

interface AlgorithmConfigurationProps {
  settings: SecuritySettings;
  onSettingsChange: (updates: Partial<SecuritySettings>) => void;
  expandedSections: Record<string, boolean>;
  onToggleSection: (section: string) => void;
}

const algorithmDetails = {
  falcon1024: {
    name: 'FALCON-1024',
    description: 'Post-quantum digital signature algorithm based on lattice cryptography',
    keySize: '1024 bits',
    securityLevel: 'NIST Level 5',
    quantumSafe: true,
    performance: 'Fast signing, moderate verification',
    standardization: 'NIST PQC Standard (2024)'
  },
  kyber768: {
    name: 'Kyber-768',
    description: 'Post-quantum key encapsulation mechanism for secure key exchange',
    keySize: '768 parameters',
    securityLevel: 'NIST Level 3',
    quantumSafe: true,
    performance: 'Fast key generation and encapsulation',
    standardization: 'NIST PQC Standard (2024)'
  }
};

export function AlgorithmConfiguration({
  settings,
  onSettingsChange,
  expandedSections,
  onToggleSection
}: AlgorithmConfigurationProps) {
  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold">Algorithm Configuration</h3>
      
      {/* FALCON-1024 Configuration */}
      <Card className="border-blue-200">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <Key className="h-5 w-5 text-blue-600" />
              <div>
                <CardTitle className="text-base">FALCON-1024 Digital Signatures</CardTitle>
                <CardDescription>Post-quantum signature algorithm</CardDescription>
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                checked={settings.falconSigning}
                onCheckedChange={(checked) => onSettingsChange({ falconSigning: checked })}
                disabled={!settings.quantumSafe}
              />
              <Collapsible 
                open={expandedSections.falcon} 
                onOpenChange={() => onToggleSection('falcon')}
              >
                <CollapsibleTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <Info className="h-4 w-4 mr-1" />
                    Details
                    {expandedSections.falcon ? (
                      <ChevronDown className="h-4 w-4 ml-1" />
                    ) : (
                      <ChevronRight className="h-4 w-4 ml-1" />
                    )}
                  </Button>
                </CollapsibleTrigger>
              </Collapsible>
            </div>
          </div>
        </CardHeader>
        <Collapsible open={expandedSections.falcon}>
          <CollapsibleContent>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Algorithm:</span>
                    <span className="font-medium">{algorithmDetails.falcon1024.name}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Key Size:</span>
                    <span className="font-medium">{algorithmDetails.falcon1024.keySize}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Security Level:</span>
                    <span className="font-medium">{algorithmDetails.falcon1024.securityLevel}</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Quantum-Safe:</span>
                    <CheckCircle className="h-4 w-4 text-green-600" />
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Performance:</span>
                    <span className="font-medium text-green-600">Fast</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Standard:</span>
                    <span className="font-medium">NIST PQC</span>
                  </div>
                </div>
              </div>
              <div className="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-lg">
                <p className="text-sm text-blue-800">
                  {algorithmDetails.falcon1024.description}
                </p>
              </div>
              <div className="mt-3 flex space-x-2">
                <Button variant="outline" size="sm">
                  <ExternalLink className="h-4 w-4 mr-1" />
                  Learn More
                </Button>
                <Button variant="outline" size="sm">
                  <Zap className="h-4 w-4 mr-1" />
                  Test Performance
                </Button>
              </div>
            </CardContent>
          </CollapsibleContent>
        </Collapsible>
      </Card>

      {/* Kyber Configuration */}
      <Card className="border-purple-200">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <Lock className="h-5 w-5 text-purple-600" />
              <div>
                <CardTitle className="text-base">Kyber Key Exchange Mechanism</CardTitle>
                <CardDescription>Post-quantum key encapsulation</CardDescription>
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                checked={settings.kyberKeyExchange}
                onCheckedChange={(checked) => onSettingsChange({ kyberKeyExchange: checked })}
                disabled={!settings.quantumSafe}
              />
              <Collapsible 
                open={expandedSections.kyber} 
                onOpenChange={() => onToggleSection('kyber')}
              >
                <CollapsibleTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <Info className="h-4 w-4 mr-1" />
                    Details
                    {expandedSections.kyber ? (
                      <ChevronDown className="h-4 w-4 ml-1" />
                    ) : (
                      <ChevronRight className="h-4 w-4 ml-1" />
                    )}
                  </Button>
                </CollapsibleTrigger>
              </Collapsible>
            </div>
          </div>
        </CardHeader>
        <Collapsible open={expandedSections.kyber}>
          <CollapsibleContent>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Algorithm:</span>
                    <span className="font-medium">{algorithmDetails.kyber768.name}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Parameters:</span>
                    <span className="font-medium">{algorithmDetails.kyber768.keySize}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Security Level:</span>
                    <span className="font-medium">{algorithmDetails.kyber768.securityLevel}</span>
                  </div>
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Quantum-Safe:</span>
                    <CheckCircle className="h-4 w-4 text-green-600" />
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Performance:</span>
                    <span className="font-medium text-green-600">Very Fast</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Standard:</span>
                    <span className="font-medium">NIST PQC</span>
                  </div>
                </div>
              </div>
              <div className="mt-4 p-3 bg-purple-50 border border-purple-200 rounded-lg">
                <p className="text-sm text-purple-800">
                  {algorithmDetails.kyber768.description}
                </p>
              </div>
            </CardContent>
          </CollapsibleContent>
        </Collapsible>
      </Card>
    </div>
  );
}