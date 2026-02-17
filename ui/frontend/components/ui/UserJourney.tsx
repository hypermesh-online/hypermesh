// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { 
  CheckCircle, 
  Circle, 
  Lock, 
  ArrowRight, 
  Trophy, 
  Star, 
  Target,
  ChevronRight,
  Shield,
  Zap,
  Network,
  Coins,
  Server,
  Gauge
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Link } from 'react-router-dom';
import { AccessibilityWrapper } from './AccessibilityWrapper';
import { NavigationElement } from './NavigationElement';
import { ScreenReaderOnly } from './ScreenReaderOnly';
import { LiveRegion } from './LiveRegion';

interface JourneyStep {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  status: 'completed' | 'current' | 'available' | 'locked';
  progress?: number;
  requirements?: string[];
  rewards?: string[];
  href: string;
  estimatedTime?: string;
  difficulty?: 'beginner' | 'intermediate' | 'advanced';
  category: 'identity' | 'network' | 'economic' | 'advanced';
}

interface Achievement {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  earned: boolean;
  earnedDate?: string;
  category: 'milestone' | 'social' | 'technical' | 'economic';
}

interface UserJourneyProps {
  className?: string;
  compact?: boolean;
  showAchievements?: boolean;
  onStepSelect?: (step: JourneyStep) => void;
}

const defaultSteps: JourneyStep[] = [
  {
    id: 'identity-setup',
    title: 'Identity Foundation',
    description: 'Establish your TrustChain identity and cryptographic keys',
    icon: Shield,
    status: 'completed',
    progress: 100,
    requirements: [],
    rewards: ['Node certificate', 'Basic network access'],
    href: '/trustchain/identity',
    estimatedTime: '5 min',
    difficulty: 'beginner',
    category: 'identity'
  },
  {
    id: 'network-connection',
    title: 'Network Access',
    description: 'Connect to STOQ transport layer for high-speed communication',
    icon: Zap,
    status: 'current',
    progress: 75,
    requirements: ['TrustChain identity'],
    rewards: ['P2P tunneling', 'Enhanced bandwidth'],
    href: '/stoq/protocol',
    estimatedTime: '10 min',
    difficulty: 'beginner',
    category: 'network'
  },
  {
    id: 'resource-sharing',
    title: 'Resource Participation',
    description: 'Share computing resources and access HyperMesh network',
    icon: Network,
    status: 'available',
    requirements: ['STOQ connection', 'Resource commitment'],
    rewards: ['CAESAR tokens', 'Network voting rights'],
    href: '/hypermesh/resources',
    estimatedTime: '15 min',
    difficulty: 'intermediate',
    category: 'network'
  },
  {
    id: 'economic-participation',
    title: 'Economic Integration',
    description: 'Join the Caesar economy and start earning tokens',
    icon: Coins,
    status: 'available',
    requirements: ['HyperMesh access'],
    rewards: ['Token wallet', 'DEX trading'],
    href: '/caesar',
    estimatedTime: '10 min',
    difficulty: 'intermediate',
    category: 'economic'
  },
  {
    id: 'asset-creation',
    title: 'Asset Management',
    description: 'Create and manage digital assets in the Catalog',
    icon: Server,
    status: 'locked',
    requirements: ['Economic participation', 'Trust score >85%'],
    rewards: ['Asset registry access', 'Service deployment'],
    href: '/catalog/creation',
    estimatedTime: '20 min',
    difficulty: 'advanced',
    category: 'advanced'
  },
  {
    id: 'analytics-mastery',
    title: 'Analytics & Insights',
    description: 'Access Ngauge analytics and contribute to user onboarding',
    icon: Gauge,
    status: 'locked',
    requirements: ['Asset management', 'Community contribution'],
    rewards: ['Analytics dashboard', 'Ad network participation'],
    href: '/ngauge',
    estimatedTime: '15 min',
    difficulty: 'advanced',
    category: 'advanced'
  }
];

const achievements: Achievement[] = [
  {
    id: 'first-steps',
    title: 'First Steps',
    description: 'Completed identity setup',
    icon: Star,
    earned: true,
    earnedDate: '2024-01-15',
    category: 'milestone'
  },
  {
    id: 'network-pioneer',
    title: 'Network Pioneer',
    description: 'Connected to 3 different network layers',
    icon: Network,
    earned: false,
    category: 'technical'
  },
  {
    id: 'token-holder',
    title: 'Token Holder',
    description: 'Earned your first 100 CAESAR tokens',
    icon: Coins,
    earned: false,
    category: 'economic'
  },
  {
    id: 'community-contributor',
    title: 'Community Contributor',
    description: 'Helped onboard 5 new users',
    icon: Trophy,
    earned: false,
    category: 'social'
  }
];

export function UserJourney({ 
  className, 
  compact = false, 
  showAchievements = true,
  onStepSelect 
}: UserJourneyProps) {
  const [steps, setSteps] = useState<JourneyStep[]>(defaultSteps);
  const [selectedStep, setSelectedStep] = useState<string | null>(null);
  const [announcement, setAnnouncement] = useState<string>('');

  const completedSteps = steps.filter(step => step.status === 'completed').length;
  const totalSteps = steps.length;
  const overallProgress = (completedSteps / totalSteps) * 100;

  const currentStep = steps.find(step => step.status === 'current');
  const nextSteps = steps.filter(step => step.status === 'available').slice(0, 2);

  useEffect(() => {
    // Simulate step progress updates
    const timer = setInterval(() => {
      setSteps(prevSteps => {
        return prevSteps.map(step => {
          if (step.status === 'current' && step.progress !== undefined && step.progress < 100) {
            const newProgress = Math.min(step.progress + Math.random() * 5, 100);
            if (newProgress === 100) {
              setAnnouncement(`${step.title} completed! You've earned new capabilities.`);
            }
            return { ...step, progress: newProgress };
          }
          return step;
        });
      });
    }, 3000);

    return () => clearInterval(timer);
  }, []);

  const handleStepClick = (step: JourneyStep) => {
    if (step.status === 'locked') {
      setAnnouncement(`${step.title} is locked. Complete the required steps first.`);
      return;
    }

    setSelectedStep(step.id);
    setAnnouncement(`Selected ${step.title}. ${step.description}`);
    onStepSelect?.(step);
  };

  const getStepIcon = (step: JourneyStep) => {
    const Icon = step.icon;
    switch (step.status) {
      case 'completed':
        return <CheckCircle className="h-6 w-6 text-green-400" />;
      case 'current':
        return <Icon className="h-6 w-6 text-cyan-400" />;
      case 'available':
        return <Circle className="h-6 w-6 text-blue-400" />;
      case 'locked':
        return <Lock className="h-6 w-6 text-gray-600" />;
    }
  };

  const getDifficultyColor = (difficulty?: string) => {
    switch (difficulty) {
      case 'beginner': return 'text-green-400';
      case 'intermediate': return 'text-yellow-400';
      case 'advanced': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  if (compact) {
    return (
      <AccessibilityWrapper
        role="region"
        ariaLabel="User journey progress"
        className={cn('space-y-4', className)}
      >
        <LiveRegion message={announcement} />
        
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-white flex items-center gap-2">
                <Target className="h-5 w-5 text-cyan-400" />
                Your Progress
              </CardTitle>
              <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30">
                {completedSteps}/{totalSteps} Complete
              </Badge>
            </div>
            <Progress value={overallProgress} className="mt-2" />
          </CardHeader>
          <CardContent>
            {currentStep && (
              <NavigationElement
                id={`current-step-${currentStep.id}`}
                order={1}
                onActivate={() => handleStepClick(currentStep)}
                ariaLabel={`Current step: ${currentStep.title}. ${currentStep.description}`}
              >
                <div className="p-3 rounded-lg bg-cyan-500/10 border border-cyan-500/20 mb-3">
                  <div className="flex items-center gap-3">
                    <div className="flex-shrink-0">
                      {getStepIcon(currentStep)}
                    </div>
                    <div className="flex-1">
                      <h4 className="font-medium text-white">{currentStep.title}</h4>
                      <p className="text-sm text-gray-400">{currentStep.description}</p>
                      {currentStep.progress !== undefined && (
                        <Progress value={currentStep.progress} className="mt-2 h-1" />
                      )}
                    </div>
                    <ChevronRight className="h-4 w-4 text-cyan-400" />
                  </div>
                </div>
              </NavigationElement>
            )}
            
            {nextSteps.length > 0 && (
              <div className="space-y-2">
                <h5 className="text-sm font-medium text-gray-300">Next Steps</h5>
                {nextSteps.map((step, index) => (
                  <NavigationElement
                    key={step.id}
                    id={`next-step-${step.id}`}
                    order={index + 2}
                    onActivate={() => handleStepClick(step)}
                    ariaLabel={`Next step: ${step.title}. ${step.description}`}
                  >
                    <div className="flex items-center gap-3 p-2 rounded hover:bg-blue-500/10 transition-colors">
                      <div className="flex-shrink-0">
                        {getStepIcon(step)}
                      </div>
                      <div className="flex-1">
                        <p className="text-sm font-medium text-white">{step.title}</p>
                        {step.estimatedTime && (
                          <p className="text-xs text-gray-500">{step.estimatedTime}</p>
                        )}
                      </div>
                    </div>
                  </NavigationElement>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </AccessibilityWrapper>
    );
  }

  return (
    <AccessibilityWrapper
      role="region"
      ariaLabel="Complete user journey through HyperMesh ecosystem"
      className={cn('space-y-6', className)}
    >
      <LiveRegion message={announcement} />
      
      {/* Journey Overview */}
      <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
        <CardHeader>
          <CardTitle className="text-white flex items-center gap-2">
            <Target className="h-6 w-6 text-cyan-400" />
            Your HyperMesh Journey
            <Badge className="ml-auto bg-cyan-500/20 text-cyan-400 border-cyan-500/30">
              {completedSteps}/{totalSteps} Steps Complete
            </Badge>
          </CardTitle>
          <div className="space-y-2">
            <Progress value={overallProgress} className="h-2" />
            <p className="text-sm text-gray-400">
              {overallProgress.toFixed(0)}% through the ecosystem
            </p>
          </div>
        </CardHeader>
      </Card>

      {/* Journey Steps */}
      <div className="grid gap-4 lg:grid-cols-2">
        {steps.map((step, index) => {
          const isSelected = selectedStep === step.id;
          const canAccess = step.status !== 'locked';
          
          return (
            <NavigationElement
              key={step.id}
              id={`journey-step-${step.id}`}
              order={index + 10}
              onActivate={() => canAccess && handleStepClick(step)}
              disabled={!canAccess}
              ariaLabel={`Step ${index + 1}: ${step.title}. Status: ${step.status}. ${step.description}`}
            >
              <Card className={cn(
                'backdrop-blur-lg transition-all duration-300 cursor-pointer',
                step.status === 'completed' ? 'bg-green-500/10 border-green-500/30' :
                step.status === 'current' ? 'bg-cyan-500/10 border-cyan-500/30' :
                step.status === 'available' ? 'bg-blue-500/10 border-blue-500/30' :
                'bg-black/20 border-gray-700/50',
                isSelected && 'ring-2 ring-cyan-400 ring-opacity-60',
                canAccess ? 'hover:shadow-lg hover:scale-[1.02]' : 'opacity-60 cursor-not-allowed'
              )}>
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="flex-shrink-0">
                        {getStepIcon(step)}
                      </div>
                      <div>
                        <CardTitle className="text-lg text-white">{step.title}</CardTitle>
                        <div className="flex items-center gap-2 mt-1">
                          {step.estimatedTime && (
                            <Badge variant="outline" className="text-xs">
                              {step.estimatedTime}
                            </Badge>
                          )}
                          {step.difficulty && (
                            <Badge variant="outline" className={cn('text-xs', getDifficultyColor(step.difficulty))}>
                              {step.difficulty}
                            </Badge>
                          )}
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex items-center gap-2">
                      {step.status === 'current' && step.progress !== undefined && (
                        <div className="text-right">
                          <p className="text-sm font-medium text-cyan-400">{Math.round(step.progress)}%</p>
                          <Progress value={step.progress} className="w-16 h-1" />
                        </div>
                      )}
                      
                      {canAccess && (
                        <Link to={step.href}>
                          <Button size="sm" variant="outline" className="border-cyan-500/30 text-cyan-400">
                            {step.status === 'completed' ? 'Revisit' : 'Continue'}
                            <ArrowRight className="h-3 w-3 ml-1" />
                          </Button>
                        </Link>
                      )}
                    </div>
                  </div>
                </CardHeader>
                
                <CardContent className="space-y-3">
                  <p className="text-gray-400">{step.description}</p>
                  
                  {step.requirements && step.requirements.length > 0 && (
                    <div>
                      <h5 className="text-sm font-medium text-gray-300 mb-1">Requirements:</h5>
                      <div className="flex flex-wrap gap-1">
                        {step.requirements.map((req, i) => (
                          <Badge key={i} variant="outline" className="text-xs bg-yellow-500/20 text-yellow-400">
                            {req}
                          </Badge>
                        ))}
                      </div>
                    </div>
                  )}
                  
                  {step.rewards && step.rewards.length > 0 && (
                    <div>
                      <h5 className="text-sm font-medium text-gray-300 mb-1">Rewards:</h5>
                      <div className="flex flex-wrap gap-1">
                        {step.rewards.map((reward, i) => (
                          <Badge key={i} variant="outline" className="text-xs bg-green-500/20 text-green-400">
                            {reward}
                          </Badge>
                        ))}
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>
            </NavigationElement>
          );
        })}
      </div>

      {/* Achievements */}
      {showAchievements && (
        <Card className="bg-black/40 border-cyan-500/30 backdrop-blur-lg">
          <CardHeader>
            <CardTitle className="text-white flex items-center gap-2">
              <Trophy className="h-6 w-6 text-yellow-400" />
              Achievements
              <Badge className="ml-auto bg-yellow-500/20 text-yellow-400 border-yellow-500/30">
                {achievements.filter(a => a.earned).length}/{achievements.length}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3 md:grid-cols-2">
              {achievements.map((achievement, index) => (
                <NavigationElement
                  key={achievement.id}
                  id={`achievement-${achievement.id}`}
                  order={index + 100}
                  ariaLabel={`Achievement: ${achievement.title}. ${achievement.earned ? 'Earned' : 'Not earned'}. ${achievement.description}`}
                >
                  <div className={cn(
                    'flex items-center gap-3 p-3 rounded-lg border transition-all duration-300',
                    achievement.earned 
                      ? 'bg-yellow-500/10 border-yellow-500/30' 
                      : 'bg-gray-500/10 border-gray-600/30'
                  )}>
                    <div className={cn(
                      'p-2 rounded-full',
                      achievement.earned ? 'bg-yellow-500/20' : 'bg-gray-500/20'
                    )}>
                      <achievement.icon className={cn(
                        'h-4 w-4',
                        achievement.earned ? 'text-yellow-400' : 'text-gray-500'
                      )} />
                    </div>
                    <div className="flex-1">
                      <h4 className={cn(
                        'font-medium',
                        achievement.earned ? 'text-white' : 'text-gray-400'
                      )}>
                        {achievement.title}
                      </h4>
                      <p className="text-sm text-gray-500">{achievement.description}</p>
                      {achievement.earned && achievement.earnedDate && (
                        <p className="text-xs text-yellow-400 mt-1">
                          Earned on {new Date(achievement.earnedDate).toLocaleDateString()}
                        </p>
                      )}
                    </div>
                    {achievement.earned && (
                      <CheckCircle className="h-5 w-5 text-yellow-400" />
                    )}
                  </div>
                </NavigationElement>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      <ScreenReaderOnly>
        <p>
          You have completed {completedSteps} out of {totalSteps} steps in your HyperMesh journey. 
          {currentStep && `Your current step is ${currentStep.title}.`}
          Use the Tab key to navigate through the steps, and press Enter to select a step.
        </p>
      </ScreenReaderOnly>
    </AccessibilityWrapper>
  );
}
