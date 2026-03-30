[CmdletBinding()]
param(
    [string]$Version = $(if ($env:VS_INSTALL_VERSION) { $env:VS_INSTALL_VERSION } else { "latest" }),
    [ValidateSet("full", "lua", "wasi")]
    [string]$Variant = $(if ($env:VS_INSTALL_VARIANT) { $env:VS_INSTALL_VARIANT } else { "full" }),
    [string]$InstallDir = $(if ($env:VS_INSTALL_DIR) { $env:VS_INSTALL_DIR } elseif ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "Programs\vs\bin" } else { Join-Path $HOME "AppData\Local\Programs\vs\bin" }),
    [string]$Target = $env:VS_INSTALL_TARGET,
    [string]$Repository = $(if ($env:VS_RELEASE_REPOSITORY) { $env:VS_RELEASE_REPOSITORY } else { "unsdk/vs" }),
    [switch]$SkipPathUpdate,
    [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Show-Usage {
    @'
Install the `vs` binary from GitHub Releases.

Usage:
  install.ps1 [-Version <latest|vX.Y.Z|X.Y.Z>] [-Variant <full|lua|wasi>]
              [-InstallDir <path>] [-Target <triple>] [-Repository <owner/name>]
              [-SkipPathUpdate]

Environment overrides:
  VS_INSTALL_VERSION
  VS_INSTALL_VARIANT
  VS_INSTALL_DIR
  VS_INSTALL_TARGET
  VS_RELEASE_REPOSITORY

Examples:
  ./scripts/install.ps1
  ./scripts/install.ps1 -Version v0.1.0 -Variant lua
  ./scripts/install.ps1 -InstallDir "$env:LOCALAPPDATA\Programs\vs\bin"
'@
}

function Write-Log {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Host "==> $Message"
}

function Normalize-Version {
    param([Parameter(Mandatory = $true)][string]$RequestedVersion)

    if ($RequestedVersion -eq "latest") {
        return Get-LatestTag
    }

    if ($RequestedVersion.StartsWith("v")) {
        return $RequestedVersion
    }

    return "v$RequestedVersion"
}

function Get-LatestTag {
    $uri = "https://api.github.com/repos/$Repository/releases/latest"
    $response = Invoke-RestMethod -Headers @{ Accept = "application/vnd.github+json" } -Uri $uri

    if (-not $response.tag_name) {
        throw "Failed to resolve the latest release tag from GitHub."
    }

    return [string]$response.tag_name
}

function Resolve-Target {
    param([string]$ExplicitTarget)

    if ($ExplicitTarget) {
        return $ExplicitTarget
    }

    $isWindows = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
    if (-not $isWindows) {
        throw "install.ps1 is intended for Windows hosts. Use scripts/install.sh on Unix-like systems."
    }

    switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()) {
        "X64" { return "x86_64-pc-windows-msvc" }
        "X86" { return "i686-pc-windows-msvc" }
        "Arm64" { return "aarch64-pc-windows-msvc" }
        default { throw "Unsupported Windows architecture: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
    }
}

function Get-ArchiveName {
    param(
        [Parameter(Mandatory = $true)][string]$Tag,
        [Parameter(Mandatory = $true)][string]$ResolvedTarget,
        [Parameter(Mandatory = $true)][string]$ResolvedVariant
    )

    return "vs-$Tag-$ResolvedTarget-$ResolvedVariant.zip"
}

function Normalize-PathSegment {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return ""
    }

    return $Value.Trim().TrimEnd([char[]]@('\'))
}

function Add-ToUserPath {
    param([Parameter(Mandatory = $true)][string]$PathToAdd)

    $normalizedPathToAdd = Normalize-PathSegment -Value $PathToAdd
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $segments = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $segments = $userPath -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    foreach ($segment in $segments) {
        if ((Normalize-PathSegment -Value $segment).Equals($normalizedPathToAdd, [System.StringComparison]::OrdinalIgnoreCase)) {
            Write-Log "$PathToAdd is already present in the user PATH."
            return
        }
    }

    $newUserPath = if ([string]::IsNullOrWhiteSpace($userPath)) {
        $PathToAdd
    } else {
        "$userPath;$PathToAdd"
    }
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")

    $processSegments = @()
    if (-not [string]::IsNullOrWhiteSpace($env:Path)) {
        $processSegments = $env:Path -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }
    $processHasPath = $false
    foreach ($segment in $processSegments) {
        if ((Normalize-PathSegment -Value $segment).Equals($normalizedPathToAdd, [System.StringComparison]::OrdinalIgnoreCase)) {
            $processHasPath = $true
            break
        }
    }
    if (-not $processHasPath) {
        $env:Path = "$PathToAdd;$env:Path"
    }

    Write-Log "Added $PathToAdd to the user PATH."
}

if ($Help) {
    Show-Usage
    exit 0
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vs-install-" + [System.Guid]::NewGuid().ToString("N"))
try {
    $tag = Normalize-Version -RequestedVersion $Version
    $resolvedTarget = Resolve-Target -ExplicitTarget $Target
    $archiveName = Get-ArchiveName -Tag $tag -ResolvedTarget $resolvedTarget -ResolvedVariant $Variant
    $archiveUrl = "https://github.com/$Repository/releases/download/$tag/$archiveName"
    $archivePath = Join-Path $tempDir $archiveName
    $extractDir = Join-Path $tempDir "extract"
    $binaryPath = Join-Path $extractDir "vs.exe"
    $destinationPath = Join-Path $InstallDir "vs.exe"

    New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

    Write-Log "Resolved release $tag"
    Write-Log "Using target $resolvedTarget ($Variant)"
    Write-Log "Downloading $archiveUrl"
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath

    Write-Log "Extracting archive"
    Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

    if (-not (Test-Path -LiteralPath $binaryPath)) {
        throw "Archive did not contain a top-level vs.exe binary."
    }

    Write-Log "Installing to $InstallDir"
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -LiteralPath $binaryPath -Destination $destinationPath -Force

    if (-not $SkipPathUpdate) {
        Add-ToUserPath -PathToAdd $InstallDir
    }

    try {
        $installedVersion = & $destinationPath --version 2>$null
        if ($installedVersion) {
            Write-Log "Installed $installedVersion"
        } else {
            Write-Log "Installed $destinationPath"
        }
    } catch {
        Write-Log "Installed $destinationPath"
    }

    if ($SkipPathUpdate) {
        Write-Host "Add $InstallDir to your PATH if it is not already available there."
    } else {
        Write-Host "Restart your terminal or PowerShell session if `vs` is not yet on PATH."
    }
} finally {
    if (Test-Path -LiteralPath $tempDir) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
