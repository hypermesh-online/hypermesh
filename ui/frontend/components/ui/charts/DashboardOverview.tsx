// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { LucideIcon, Settings, Maximize2, RefreshCw } from 'lucide-react';
import { TopologyChart } from './TopologyChart';
import type { TopologyNode, TopologyLink } from './TopologyChart';
import { PerformanceChart } from './PerformanceChart';
import type { PerformanceMetric } from './PerformanceChart';
import { SystemMetrics } from './SystemMetrics';
import { MetricDisplay } from './MetricDisplay';

interface DashboardWidget {
  id: string;
  title: string;
  type: 'topology' | 'performance' | 'metrics' | 'kpi';
  size: 'small' | 'medium' | 'large' | 'full';
  position: { x: number; y: number };
  config: any;
  refreshInterval?: number;
}

interface DashboardOverviewProps {
  widgets: DashboardWidget[];
  theme?: 'cyan' | 'green' | 'purple' | 'red' | 'yellow';
  layout?: 'grid' | 'masonry' | 'custom';
  editable?: boolean;
  autoRefresh?: boolean;
  onWidgetClick?: (widget: DashboardWidget) => void;
  onWidgetEdit?: (widget: DashboardWidget) => void;
  className?: string;
}

export function DashboardOverview({
  widgets,
  theme = 'cyan',
  layout = 'grid',
  editable = false,
  autoRefresh = true,
  onWidgetClick,
  onWidgetEdit,
  className
}: DashboardOverviewProps) {
  const getWidgetSize = (size: string) => {
    switch (size) {
      case 'small': return 'col-span-1 row-span-1';
      case 'medium': return 'col-span-2 row-span-1';
      case 'large': return 'col-span-2 row-span-2';
      case 'full': return 'col-span-4 row-span-2';
      default: return 'col-span-1 row-span-1';
    }
  };

  const renderWidget = (widget: DashboardWidget) => {
    const baseProps = {
      theme,
      className: 'w-full h-full'
    };

    switch (widget.type) {
      case 'topology':
        return (
          <TopologyChart
            nodes={widget.config.nodes || []}
            links={widget.config.links || []}
            layout={widget.config.layout || 'force'}
            width={widget.size === 'large' || widget.size === 'full' ? 600 : 400}
            height={widget.size === 'large' || widget.size === 'full' ? 400 : 300}
            showMetrics={widget.config.showMetrics !== false}
            showLabels={widget.config.showLabels !== false}
            {...baseProps}
          />
        );

      case 'performance':
        return (
          <PerformanceChart
            metrics={widget.config.metrics || []}
            type={widget.config.type || 'line'}
            height={widget.size === 'large' || widget.size === 'full' ? 300 : 200}
            timeRange={widget.config.timeRange || '24h'}
            showLegend={widget.config.showLegend !== false}
            realtime={widget.config.realtime || false}
            {...baseProps}
          />
        );

      case 'metrics':
        return (
          <SystemMetrics
            metrics={widget.config.metrics || []}
            layout={widget.config.layout || 'grid'}
            showTrends={widget.config.showTrends !== false}
            showHistory={widget.config.showHistory !== false}
            showGauges={widget.config.showGauges || false}
            groupByCategory={widget.config.groupByCategory || false}
            {...baseProps}
          />
        );

      case 'kpi':
        return (
          <div className="grid gap-4 h-full">
            {(widget.config.kpis || []).map((kpi: any, index: number) => (
              <MetricDisplay
                key={index}
                title={kpi.title}
                value={kpi.value}
                subtitle={kpi.subtitle}
                icon={kpi.icon}
                trend={kpi.trend}
                progress={kpi.progress}
                status={kpi.status}
                size={widget.size === 'small' ? 'sm' : 'md'}
                {...baseProps}
              />
            ))}
          </div>
        );

      default:
        return (
          <div className="flex items-center justify-center h-full text-gray-400">
            <p>Unknown widget type: {widget.type}</p>
          </div>
        );
    }
  };

  return (
    <div className={cn('space-y-6', className)}>
      {/* Dashboard controls */}
      {editable && (
        <div className="flex justify-between items-center">
          <h2 className="text-2xl font-bold text-white">Dashboard Overview</h2>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" className="border-gray-600 text-gray-300">
              <Settings className="h-4 w-4 mr-2" />
              Configure
            </Button>
            <Button variant="outline" size="sm" className="border-gray-600 text-gray-300">
              <Maximize2 className="h-4 w-4 mr-2" />
              Fullscreen
            </Button>
            <Button variant="outline" size="sm" className="border-gray-600 text-gray-300">
              <RefreshCw className="h-4 w-4 mr-2" />
              Refresh
            </Button>
          </div>
        </div>
      )}

      {/* Widgets grid */}
      <div className={cn(
        layout === 'grid' && 'grid grid-cols-4 gap-6 auto-rows-fr',
        layout === 'masonry' && 'columns-4 gap-6',
        layout === 'custom' && 'relative'
      )}>
        {widgets.map((widget) => (
          <Card
            key={widget.id}
            className={cn(
              'bg-black/40 backdrop-blur-lg border-gray-700 transition-all duration-300 hover:shadow-lg',
              layout === 'grid' && getWidgetSize(widget.size),
              layout === 'masonry' && 'break-inside-avoid mb-6',
              layout === 'custom' && 'absolute',
              'min-h-[200px]'
            )}
            style={layout === 'custom' ? {
              left: `${widget.position.x}px`,
              top: `${widget.position.y}px`,
              width: widget.size === 'large' || widget.size === 'full' ? '600px' : '300px',
              height: widget.size === 'large' || widget.size === 'full' ? '400px' : '200px'
            } : undefined}
          >
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-lg text-white">{widget.title}</CardTitle>
                
                {editable && (
                  <div className="flex gap-1">
                    {autoRefresh && widget.refreshInterval && (
                      <div className="flex items-center gap-1 text-xs text-gray-400">
                        <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse" />
                        <span>{widget.refreshInterval}s</span>
                      </div>
                    )}
                    
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 hover:bg-gray-800"
                      onClick={() => onWidgetEdit?.(widget)}
                    >
                      <Settings className="h-3 w-3" />
                    </Button>
                    
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 hover:bg-gray-800"
                      onClick={() => onWidgetClick?.(widget)}
                    >
                      <Maximize2 className="h-3 w-3" />
                    </Button>
                  </div>
                )}
              </div>
            </CardHeader>
            
            <CardContent className="pt-0 h-full">
              <div className="h-full">
                {renderWidget(widget)}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Empty state */}
      {widgets.length === 0 && (
        <div className="flex flex-col items-center justify-center h-64 text-gray-400">
          <p className="text-lg mb-2">No widgets configured</p>
          <p className="text-sm">Add widgets to start monitoring your system</p>
          {editable && (
            <Button className="mt-4" onClick={() => console.log('Add widget')}>
              Add Widget
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
