# Install HyperMesh node from GitHub Releases (Windows / PowerShell).
# Usage:
#   iwr -useb https://raw.githubusercontent.com/hypermesh-online/core/main/scripts/install.ps1 | iex
#
# Or with arguments (download first, then run):
#   .\install.ps1 -Version v0.2.0 -Prefix "$env:LOCALAPPDATA\HyperMesh"
#
# Parameters:
#   -Version <tag>    Install specific version (default: latest)
#   -Prefix <dir>     Install prefix (default: %ProgramFiles%\HyperMesh, fallback to %LOCALAPPDATA%\HyperMesh)
#   -NoVerify         Skip SHA-256 verification against release-manifest.json
#   -Target <triple>  Force a specific Rust target triple (default: auto-detected)
#
# After install, use `hypermesh update` (Phase J release_feed_subscriber) to
# upgrade — install.ps1 is bootstrap-only. Subsequent updates flow through the
# foundation-signed release feed.

[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$Prefix = "",
    [switch]$NoVerify,
    [string]$Target = ""
)

$ErrorActionPreference = "Stop"

$GitHubOrg  = "hypermesh-online"
$GitHubRepo = "hypermesh"

# ---------------------------------------------------------------------------
# Detect target triple (Rust convention)
# ---------------------------------------------------------------------------
function Get-DefaultTarget {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if (-not $arch) {
        $arch = (Get-CimInstance Win32_Processor | Select-Object -First 1).Architecture
    }

    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        9       { return "x86_64-pc-windows-msvc" }   # Win32_Processor.Architecture: 9 = x64
        12      { return "aarch64-pc-windows-msvc" }  # Win32_Processor.Architecture: 12 = arm64
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

if (-not $Target) { $Target = Get-DefaultTarget }

# ---------------------------------------------------------------------------
# Determine install prefix
# ---------------------------------------------------------------------------
if (-not $Prefix) {
    $programFiles = $env:ProgramFiles
    if (-not $programFiles) { $programFiles = "C:\Program Files" }

    $candidate = Join-Path $programFiles "HyperMesh"
    try {
        # Test writability by creating + deleting a sentinel file.
        if (-not (Test-Path $candidate)) {
            New-Item -ItemType Directory -Path $candidate -Force -ErrorAction Stop | Out-Null
        }
        $sentinel = Join-Path $candidate ".write_test"
        New-Item -ItemType File -Path $sentinel -Force -ErrorAction Stop | Out-Null
        Remove-Item $sentinel -Force
        $Prefix = $candidate
    } catch {
        # Fall back to user-writable LocalAppData when not running elevated.
        $Prefix = Join-Path $env:LOCALAPPDATA "HyperMesh"
        Write-Host "  (Program Files not writable — installing to user profile: $Prefix)"
    }
}

$BinDir = Join-Path $Prefix "bin"
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null

# ---------------------------------------------------------------------------
# Determine version
# ---------------------------------------------------------------------------
if (-not $Version) {
    Write-Host "Fetching latest release..."
    try {
        $latest = Invoke-RestMethod "https://api.github.com/repos/$GitHubOrg/$GitHubRepo/releases/latest"
        $Version = $latest.tag_name
    } catch {
        Write-Error "Could not determine latest version. Use -Version to specify."
        exit 1
    }
}

if (-not $Version) {
    Write-Error "No version resolved."
    exit 1
}

Write-Host "Installing HyperMesh $Version for $Target..."

$Archive       = "hypermesh-$Version-$Target.zip"
$DownloadUrl   = "https://github.com/$GitHubOrg/$GitHubRepo/releases/download/$Version/$Archive"
$ManifestUrl   = "https://github.com/$GitHubOrg/$GitHubRepo/releases/download/$Version/release-manifest.json"

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

try {
    # ---------------------------------------------------------------------------
    # Download archive
    # ---------------------------------------------------------------------------
    $archivePath = Join-Path $TmpDir $Archive
    Write-Host "Downloading $DownloadUrl..."
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $archivePath -UseBasicParsing
    } catch {
        Write-Error "Failed to download $Archive. Check that release $Version includes a build for $Target at: https://github.com/$GitHubOrg/$GitHubRepo/releases/tag/$Version"
        exit 1
    }

    # ---------------------------------------------------------------------------
    # SHA-256 verification against release-manifest.json
    # ---------------------------------------------------------------------------
    $extractRoot = Join-Path $TmpDir "extracted"
    New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null
    Expand-Archive -Path $archivePath -DestinationPath $extractRoot -Force

    # Locate the binary (the archive contains a hypermesh-<ver>-<target>/ folder)
    $stageDir = Join-Path $extractRoot "hypermesh-$Version-$Target"
    if (-not (Test-Path $stageDir)) {
        $stageDir = $extractRoot
    }
    $binPath = Join-Path $stageDir "hypermesh.exe"
    if (-not (Test-Path $binPath)) {
        # Fall back to scanning for the .exe at any depth.
        $found = Get-ChildItem -Path $extractRoot -Filter "hypermesh.exe" -Recurse | Select-Object -First 1
        if ($found) { $binPath = $found.FullName }
    }
    if (-not (Test-Path $binPath)) {
        Write-Error "hypermesh.exe not found in archive."
        exit 1
    }

    if (-not $NoVerify) {
        Write-Host "Verifying SHA-256 against release-manifest.json..."
        $manifestPath = Join-Path $TmpDir "release-manifest.json"
        try {
            Invoke-WebRequest -Uri $ManifestUrl -OutFile $manifestPath -UseBasicParsing
            $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json

            $expected = $null
            if ($manifest.binary_hashes -and $manifest.binary_hashes.PSObject.Properties[$Target]) {
                $expected = $manifest.binary_hashes.$Target
            }

            if (-not $expected) {
                Write-Warning "No hash entry for $Target in release-manifest.json — skipping verification."
            } else {
                $actual = (Get-FileHash -Algorithm SHA256 -Path $binPath).Hash.ToLower()
                if ($actual -ne $expected.ToLower()) {
                    Write-Error "SHA-256 mismatch for hypermesh.exe ($Target):`n  expected: $expected`n  actual:   $actual"
                    exit 1
                }
                Write-Host "  OK ($actual)"
            }

            # Foundation signature check is not done by install.ps1. The
            # release_feed subscriber inside the running daemon validates
            # FALCON-1024 signatures against the configured foundation pubkey.
            if ($manifest.signature -eq "") {
                Write-Host "  NOTE: release-manifest.json is not yet foundation-signed."
                Write-Host "        Daemon FALCON-1024 verification will run on first feed poll."
            }
        } catch {
            Write-Warning "release-manifest.json not found in release — skipping verification."
        }
    }

    # ---------------------------------------------------------------------------
    # Install binary
    # ---------------------------------------------------------------------------
    $dest = Join-Path $BinDir "hypermesh.exe"
    Copy-Item -Path $binPath -Destination $dest -Force
    Write-Host "  Installed hypermesh.exe -> $dest"

    # ---------------------------------------------------------------------------
    # Add to PATH (User-level, persists across sessions)
    # ---------------------------------------------------------------------------
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) { $userPath = "" }
    $pathParts = $userPath.Split(';') | Where-Object { $_ -and ($_.TrimEnd('\').ToLower() -ne $BinDir.TrimEnd('\').ToLower()) }
    if (($userPath.Split(';') | Where-Object { $_.TrimEnd('\').ToLower() -eq $BinDir.TrimEnd('\').ToLower() }).Count -eq 0) {
        $newPath = if ($pathParts) { ($pathParts -join ';') + ';' + $BinDir } else { $BinDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "  Added $BinDir to user PATH (open a new shell to pick it up)."
    } else {
        Write-Host "  $BinDir already on user PATH."
    }

    # Update current-session PATH too, so the user can run hypermesh immediately.
    if (-not (($env:Path -split ';') -contains $BinDir)) {
        $env:Path = "$env:Path;$BinDir"
    }
} finally {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "=== HyperMesh $Version installed for $Target ==="
Write-Host "Binary:   $BinDir\hypermesh.exe"
Write-Host ""
Write-Host "Future updates: use ``hypermesh update`` (foundation release feed)."
