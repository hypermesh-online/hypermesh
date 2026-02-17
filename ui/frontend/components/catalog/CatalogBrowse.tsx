// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { 
  Package, 
  Download,
  Play,
  CheckCircle,
  AlertTriangle,
  Clock,
  Eye,
  Cpu,
  Database,
  Globe,
  Shield,
  Activity,
  Zap
} from 'lucide-react';
import {
  useCatalogApplications,
  useInstallCatalogApplication,
  useExecuteVMAsset
} from '@/lib/api/hooks/useAssets';
import {
  CatalogApplication,
  PrivacyLevel
} from '@/lib/api/services/HyperMeshAPI';
import { CatalogSearchHeader } from './CatalogSearchHeader';
import { ApplicationCard } from './ApplicationCard';

interface CatalogBrowseProps {
  selectedPrivacyLevel: PrivacyLevel;
  onPrivacyLevelChange: (level: PrivacyLevel) => void;
}

export function CatalogBrowse({ selectedPrivacyLevel, onPrivacyLevelChange }: CatalogBrowseProps) {
  const { applications, isLoading, availableApps, installedApps, vmAssets } = useCatalogApplications();
  const installApp = useInstallCatalogApplication();
  const executeVM = useExecuteVMAsset();

  const handleInstallApplication = async (app: CatalogApplication) => {
    try {
      await installApp.mutateAsync({
        catalogId: app.id,
        config: {
          privacyLevel: selectedPrivacyLevel,
          autoStart: false,
          resourceLimits: {
            maxCpu: app.requirements.cpu || 1,
            maxMemory: `${app.requirements.memory || 1}GB`,
            maxStorage: `${app.requirements.storage || 1}GB`,
            maxExecutionTime: 300
          }
        }
      });
      alert(`Successfully installed ${app.name} as HyperMesh VM asset!`);
    } catch (error) {
      console.error('Installation failed:', error);
      alert(`Failed to install ${app.name}. Check console for details.`);
    }
  };

  const handleRunApplication = async (app: CatalogApplication) => {
    if (!app.assetId) {
      alert('Application must be installed as VM asset first');
      return;
    }

    try {
      await executeVM.mutateAsync({
        vmAssetId: app.assetId,
        operation: 'start',
        parameters: {},
        timeout: 300,
        requiresConsensus: true,
        allocationDuration: 3600
      });
      alert(`Starting ${app.name} execution through HyperMesh...`);
    } catch (error) {
      console.error('Execution failed:', error);
      alert(`Failed to run ${app.name}. Check console for details.`);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <CatalogSearchHeader 
          selectedPrivacyLevel={selectedPrivacyLevel}
          onPrivacyLevelChange={onPrivacyLevelChange}
        />
        <div className="text-center py-12">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-400 mx-auto"></div>
          <p className="text-gray-400 mt-4">Loading HyperMesh assets...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <CatalogSearchHeader 
        selectedPrivacyLevel={selectedPrivacyLevel}
        onPrivacyLevelChange={onPrivacyLevelChange}
      />

      {/* Browse Section */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        {availableApps.map((app) => (
          <ApplicationCard
            key={app.id}
            app={app}
            onInstall={handleInstallApplication}
            onRun={handleRunApplication}
            isInstalling={installApp.isPending}
            isRunning={executeVM.isPending}
          />
        ))}
      </div>

      {availableApps.length === 0 && (
        <div className="text-center py-12">
          <Package className="h-16 w-16 text-gray-600 mx-auto mb-4" />
          <h3 className="text-xl font-semibold text-gray-300 mb-2">No Applications Available</h3>
          <p className="text-gray-400">No applications match your current search criteria.</p>
        </div>
      )}
    </div>
  );
}