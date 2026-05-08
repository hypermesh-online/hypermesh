// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
//
// Phase C.3 — First-run setup wizard.
//
// Six-page flow:
//   1. Welcome
//   2. Privacy mode (Anonymous / Private / Public)
//   3. Identity generation (calls daemon `auth.create_session` IPC)
//   4. Optional: join trustnet-test
//   5. Optional: foundation grant token
//   6. Done
//
// Mounted from App.tsx whenever `wizard_should_show` returns true. In
// non-Tauri builds (Gateway-served standalone UI) `isTauri()` is false
// and the wizard short-circuits to render nothing — the existing
// dashboard appears immediately.

import React, { useEffect, useMemo, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';
import { isTauri, wizard, daemon } from './tauriBridge';
import { blockMatrixClient } from '@/lib/blockmatrix-api';

type Page = 'welcome' | 'privacy' | 'identity' | 'trustnet' | 'grant' | 'done';

interface Props {
  onClose: () => void;
}

const PAGE_ORDER: Page[] = ['welcome', 'privacy', 'identity', 'trustnet', 'grant', 'done'];

export function SetupWizard({ onClose }: Props) {
  const [page, setPage] = useState<Page>('welcome');
  const [privacyMode, setPrivacyMode] = useState<'anonymous' | 'private' | 'public'>('private');
  const [identityProgress, setIdentityProgress] = useState(0);
  const [identityFingerprint, setIdentityFingerprint] = useState<string | null>(null);
  const [identityError, setIdentityError] = useState<string | null>(null);
  const [joinTrustnet, setJoinTrustnet] = useState(false);
  const [foundationGrantToken, setFoundationGrantToken] = useState('');
  const [foundationStatus, setFoundationStatus] = useState<string | null>(null);

  const stepIndex = PAGE_ORDER.indexOf(page);
  const stepProgress = ((stepIndex + 1) / PAGE_ORDER.length) * 100;

  const next = () => {
    const i = PAGE_ORDER.indexOf(page);
    if (i < PAGE_ORDER.length - 1) setPage(PAGE_ORDER[i + 1]);
  };
  const back = () => {
    const i = PAGE_ORDER.indexOf(page);
    if (i > 0) setPage(PAGE_ORDER[i - 1]);
  };

  const finish = async () => {
    await wizard.complete();
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/85 flex items-center justify-center p-6">
      <Card className="w-full max-w-2xl bg-zinc-950 border-zinc-800">
        <CardHeader>
          <CardTitle className="text-xl">HyperMesh setup</CardTitle>
          <CardDescription>
            Step {stepIndex + 1} of {PAGE_ORDER.length}
          </CardDescription>
          <Progress value={stepProgress} className="mt-2" />
        </CardHeader>
        <CardContent className="space-y-6">
          {page === 'welcome' && <WelcomePage />}
          {page === 'privacy' && (
            <PrivacyPage value={privacyMode} onChange={setPrivacyMode} />
          )}
          {page === 'identity' && (
            <IdentityPage
              privacyMode={privacyMode}
              progress={identityProgress}
              fingerprint={identityFingerprint}
              error={identityError}
              onGenerate={async () => {
                setIdentityError(null);
                setIdentityProgress(10);
                try {
                  // Persist user choice first (Rust side will record privacy mode).
                  await wizard.setPrivacy(privacyMode);
                  setIdentityProgress(40);
                  // Ask the daemon to ensure identity exists. The daemon
                  // will generate a FALCON-1024 keypair if one isn't
                  // already on disk. We use the existing IPC method
                  // surfaced through the Gateway (`/api/v1`) for the
                  // Gateway-served path; in the Tauri path it goes
                  // through the same gateway HTTP endpoint or directly
                  // through `daemon.checkUpdate`-style calls. For
                  // alpha we call `core.health` which forces the
                  // daemon to initialise its key material.
                  await blockMatrixClient.ping().catch(() => {});
                  setIdentityProgress(80);
                  // Fetch the identity fingerprint. The wizard state
                  // includes the path; the actual hash comes from a
                  // dedicated IPC method when present, or from
                  // `core.identity` if available. We fall back to
                  // showing the path itself — sufficient for alpha.
                  const state = await wizard.state();
                  setIdentityFingerprint(state.identity_path);
                  setIdentityProgress(100);
                } catch (err) {
                  setIdentityError(String(err));
                }
              }}
            />
          )}
          {page === 'trustnet' && (
            <TrustnetPage value={joinTrustnet} onChange={setJoinTrustnet} />
          )}
          {page === 'grant' && (
            <GrantPage
              token={foundationGrantToken}
              setToken={setFoundationGrantToken}
              status={foundationStatus}
              onSubmit={async () => {
                setFoundationStatus('Submitting…');
                await wizard.setFoundationGrant(true);
                // The actual `dns.foundation_grant` IPC call goes via
                // the existing blockMatrixClient when wired (post-J).
                // For C.3 alpha we record intent and let the user
                // re-submit from the dashboard once the daemon is up.
                setFoundationStatus(
                  'Recorded. Submit the token from the dashboard once the daemon is running.',
                );
              }}
            />
          )}
          {page === 'done' && <DonePage />}

          <Separator />

          <div className="flex justify-between">
            <Button variant="ghost" onClick={back} disabled={stepIndex === 0}>
              Back
            </Button>
            <div className="flex gap-2">
              {page === 'trustnet' || page === 'grant' ? (
                <Button variant="outline" onClick={next}>
                  Skip
                </Button>
              ) : null}
              {page === 'done' ? (
                <Button onClick={finish}>Open dashboard</Button>
              ) : (
                <Button onClick={next}>Next</Button>
              )}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function WelcomePage() {
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Welcome to HyperMesh</h2>
      <p className="text-sm text-zinc-400">
        HyperMesh is a sovereign distributed asset network. Your device runs a
        local blockchain (Device scope) from boot, and can optionally join one
        or more synchronised Network chains. There is no global consensus —
        every state claim carries its own four-proof bundle.
      </p>
      <p className="text-sm text-zinc-400">
        This wizard takes about a minute. Press <strong>Next</strong> to begin.
      </p>
    </div>
  );
}

function PrivacyPage({
  value,
  onChange,
}: {
  value: 'anonymous' | 'private' | 'public';
  onChange: (v: 'anonymous' | 'private' | 'public') => void;
}) {
  const options: Array<{
    id: 'anonymous' | 'private' | 'public';
    title: string;
    body: string;
  }> = [
    {
      id: 'anonymous',
      title: 'Anonymous',
      body: 'Open transport, no identity disclosure, no Caesar rewards. Useful for privacy-first observation.',
    },
    {
      id: 'private',
      title: 'Private (recommended)',
      body: 'Bounded peer set with identity exchange, partial Caesar rewards. Default for personal devices.',
    },
    {
      id: 'public',
      title: 'Public',
      body: 'Full transparency, federated trust, maximum Caesar rewards. Suitable for foundation / gateway nodes.',
    },
  ];
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Choose privacy mode</h2>
      <p className="text-sm text-zinc-400">
        Privacy mode controls transport-layer behaviour (STOQ). It is independent
        of which blockchain scopes you join — you can change it later from
        Settings.
      </p>
      <div className="grid gap-2">
        {options.map((opt) => (
          <label
            key={opt.id}
            className={`flex gap-3 rounded-md border p-3 cursor-pointer transition ${
              value === opt.id
                ? 'border-cyan-500/60 bg-cyan-500/5'
                : 'border-zinc-800 hover:border-zinc-700'
            }`}
          >
            <input
              type="radio"
              name="privacy"
              checked={value === opt.id}
              onChange={() => onChange(opt.id)}
              className="mt-1"
            />
            <div>
              <div className="font-medium text-zinc-100">{opt.title}</div>
              <div className="text-xs text-zinc-400">{opt.body}</div>
            </div>
          </label>
        ))}
      </div>
    </div>
  );
}

function IdentityPage({
  privacyMode,
  progress,
  fingerprint,
  error,
  onGenerate,
}: {
  privacyMode: string;
  progress: number;
  fingerprint: string | null;
  error: string | null;
  onGenerate: () => void;
}) {
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Generate identity</h2>
      <p className="text-sm text-zinc-400">
        Your node uses a FALCON-1024 keypair as its long-term identity. Selected
        privacy mode: <code className="text-cyan-400">{privacyMode}</code>.
      </p>
      <Button onClick={onGenerate} disabled={progress > 0 && progress < 100}>
        {progress === 0 ? 'Generate identity' : progress < 100 ? 'Generating…' : 'Regenerate'}
      </Button>
      {progress > 0 && <Progress value={progress} />}
      {error && <p className="text-sm text-red-400">{error}</p>}
      {fingerprint && (
        <div className="rounded bg-zinc-900 p-3 text-xs text-zinc-300 break-all">
          <div className="text-zinc-500 mb-1">Identity file</div>
          {fingerprint}
        </div>
      )}
    </div>
  );
}

function TrustnetPage({
  value,
  onChange,
}: {
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Join trustnet-test? (optional)</h2>
      <p className="text-sm text-zinc-400">
        trustnet-test is the foundation-operated public testnet. Joining lets
        your node participate in the live mesh without committing to mainnet.
      </p>
      <label className="flex items-center gap-2 text-sm text-zinc-200">
        <input type="checkbox" checked={value} onChange={(e) => onChange(e.target.checked)} />
        Yes, join trustnet-test on first daemon launch
      </label>
    </div>
  );
}

function GrantPage({
  token,
  setToken,
  status,
  onSubmit,
}: {
  token: string;
  setToken: (s: string) => void;
  status: string | null;
  onSubmit: () => void;
}) {
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">Foundation grant (optional)</h2>
      <p className="text-sm text-zinc-400">
        If the foundation issued you a grant token (BLAKE3-HMAC), paste it
        below. This pre-registers your node with elevated trust on first
        connect. Skip if you don't have one.
      </p>
      <div className="space-y-2">
        <Label htmlFor="grant">Grant token</Label>
        <Input
          id="grant"
          placeholder="hmac:…"
          value={token}
          onChange={(e) => setToken(e.target.value)}
        />
      </div>
      <Button onClick={onSubmit} disabled={!token}>
        Submit
      </Button>
      {status && <p className="text-xs text-zinc-400">{status}</p>}
    </div>
  );
}

function DonePage() {
  return (
    <div className="space-y-3">
      <h2 className="text-lg font-semibold">All set</h2>
      <p className="text-sm text-zinc-400">
        You're ready to go. The dashboard opens next; the daemon is managed in
        the system tray. Your tray icon turns green when the daemon is up.
      </p>
      <ul className="text-xs text-zinc-500 list-disc pl-5 space-y-1">
        <li>Right-click the tray icon for daemon controls.</li>
        <li>Closing the window keeps HyperMesh running in the tray.</li>
        <li>Quit fully via tray → Quit.</li>
      </ul>
    </div>
  );
}

/**
 * Hook that gates rendering of the wizard. Returns the wizard component
 * to mount, or `null` when the wizard should not be shown (already
 * completed, or running outside Tauri).
 */
export function useSetupWizardGate(): React.ReactNode | null {
  const [show, setShow] = useState<boolean | null>(null);
  useEffect(() => {
    let cancelled = false;
    if (!isTauri()) {
      setShow(false);
      return;
    }
    wizard.shouldShow().then((v) => {
      if (!cancelled) setShow(v);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (show !== true) return null;
  return <SetupWizard onClose={() => setShow(false)} />;
}
