<#
.SYNOPSIS
    Generates or verifies SHA-256 checksums (SHA256SUMS.txt) for ORBIT release binaries.

.DESCRIPTION
    Scans the specified directory for release artifacts (.exe, .msi, .zip),
    computes SHA-256 hashes, and outputs standard Unix-style checksum files.
    Can also verify an existing SHA256SUMS.txt against local binaries.

.PARAMETER Directory
    The folder containing release binaries. Default: "release"

.PARAMETER OutputFile
    The checksum output file path. Default: "<Directory>\SHA256SUMS.txt"

.PARAMETER Verify
    If set, validates existing binaries against the specified checksum file.

.EXAMPLE
    .\scripts\generate-checksums.ps1
    .\scripts\generate-checksums.ps1 -Directory "release"
    .\scripts\generate-checksums.ps1 -Directory "release" -Verify
#>

param(
    [string]$Directory = "release",
    [string]$OutputFile = "",
    [switch]$Verify
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir

# Resolve relative path against repo root if not absolute
if (-not [System.IO.Path]::IsPathRooted($Directory)) {
    $targetDir = Join-Path $repoRoot $Directory
} else {
    $targetDir = $Directory
}

if (-not (Test-Path $targetDir)) {
    # Check fallback: src-tauri\target\release
    $fallbackDir = Join-Path $repoRoot "src-tauri\target\release"
    if (Test-Path $fallbackDir) {
        Write-Host "Target directory '$targetDir' not found. Falling back to '$fallbackDir'." -ForegroundColor Yellow
        $targetDir = $fallbackDir
    } else {
        Write-Error "Target directory does not exist: $targetDir"
        exit 1
    }
}

if ([string]::IsNullOrWhiteSpace($OutputFile)) {
    $OutputFile = Join-Path $targetDir "SHA256SUMS.txt"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputFile)) {
    $OutputFile = Join-Path $repoRoot $OutputFile
}

if ($Verify) {
    if (-not (Test-Path $OutputFile)) {
        Write-Error "Verification failed: Checksum file not found at $OutputFile"
        exit 1
    }

    Write-Host "Verifying release binaries against $OutputFile..." -ForegroundColor Cyan
    $lines = Get-Content $OutputFile | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and -not $_.StartsWith("#") }
    $allPassed = $true

    foreach ($line in $lines) {
        # Format: <hash>  <filename>
        $parts = $line -split '\s+', 2
        if ($parts.Length -lt 2) { continue }
        $expectedHash = $parts[0].Trim().ToLowerInvariant()
        $fileName = $parts[1].Trim()

        $filePath = Join-Path $targetDir $fileName
        if (-not (Test-Path $filePath)) {
            Write-Host " [MISSING] $fileName" -ForegroundColor Red
            $allPassed = $false
            continue
        }

        $actualHash = (Get-FileHash -Path $filePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualHash -eq $expectedHash) {
            Write-Host " [OK]      $fileName" -ForegroundColor Green
        } else {
            Write-Host " [FAILED]  $fileName (expected $expectedHash, got $actualHash)" -ForegroundColor Red
            $allPassed = $false
        }
    }

    if ($allPassed) {
        Write-Host "`nAll checksums verified successfully!" -ForegroundColor Green
        exit 0
    } else {
        Write-Host "`nChecksum verification failed for one or more files!" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Generating SHA-256 checksums for files in: $targetDir" -ForegroundColor Cyan

    $files = Get-ChildItem -Path $targetDir -File |
        Where-Object { ($_.Extension -in @(".exe", ".msi", ".zip")) -and ($_.Name -ne "SHA256SUMS.txt") }

    if ($files.Count -eq 0) {
        Write-Host "No binaries matching (*.exe, *.msi, *.zip) found in $targetDir." -ForegroundColor Yellow
        exit 0
    }

    $checksumEntries = @()
    foreach ($file in $files) {
        $hashResult = Get-FileHash -Path $file.FullName -Algorithm SHA256
        $hashLower = $hashResult.Hash.ToLowerInvariant()
        $entry = "$hashLower  $($file.Name)"
        $checksumEntries += $entry

        $sizeKb = [math]::Round($file.Length / 1KB, 1)
        Write-Host "  $hashLower  $($file.Name) (${sizeKb} KB)" -ForegroundColor Gray
    }

    $checksumEntries | Out-File -FilePath $OutputFile -Encoding utf8 -Force
    Write-Host "`nSHA-256 checksums written to: $OutputFile" -ForegroundColor Green
}
