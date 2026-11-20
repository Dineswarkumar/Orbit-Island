<#
.SYNOPSIS
    ORBIT Gate measurement script (Section 8 of ORBIT_EGUI_MIGRATION_PLAN.md).

.DESCRIPTION
    Samples process memory (private commit, private working set, working set)
    and 1-core CPU utilization every 60 seconds for 5 minutes (default).
    Records the exact 16 required CSV columns and prints the mean and range
    after discarding the first sample.

.EXAMPLE
    .\measure\measure.ps1 -Condition A -ExePath .\egui-app\target\release\orbit-egui.exe
.EXAMPLE
    .\measure\measure.ps1 -Condition B -ExePath .\egui-app\target\release\orbit-egui.exe -ExtraArgs "--hidden" -StopAfter
#>
[CmdletBinding()]
param(
    [string]$ProcessName = "orbit-egui",
    [ValidateSet("A", "B", "C", "D", "E", "F")]
    [string]$Condition = "A",
    [int]$DurationSec = 300,
    [int]$IntervalSec = 60,
    [string]$OutputCsv = "measure/results.csv",
    [string]$ExePath = "",
    [string[]]$ExtraArgs = @(),
    [switch]$StopAfter
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class OrbitMemNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct PROCESS_MEMORY_COUNTERS_EX {
        public uint cb;
        public uint PageFaultCount;
        public UIntPtr PeakWorkingSetSize;
        public UIntPtr WorkingSetSize;
        public UIntPtr QuotaPeakPagedPoolUsage;
        public UIntPtr QuotaPagedPoolUsage;
        public UIntPtr QuotaPeakNonPagedPoolUsage;
        public UIntPtr QuotaNonPagedPoolUsage;
        public UIntPtr PagefileUsage;
        public UIntPtr PeakPagefileUsage;
        public UIntPtr PrivateUsage;
    }

    [DllImport("psapi.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetProcessMemoryInfo(
        IntPtr hProcess,
        out PROCESS_MEMORY_COUNTERS_EX ppsmemCounters,
        uint cb
    );
}
"@

function Get-ProcessPrivateCommitBytes([System.Diagnostics.Process]$Proc) {
    try {
        $counters = New-Object OrbitMemNative+PROCESS_MEMORY_COUNTERS_EX
        $counters.cb = [uint32][System.Runtime.InteropServices.Marshal]::SizeOf($counters)
        if ([OrbitMemNative]::GetProcessMemoryInfo($Proc.Handle, [ref]$counters, $counters.cb)) {
            return [double]$counters.PrivateUsage.ToUInt64()
        }
    } catch {
        # Fallback below if handle access is restricted
    }
    return [double]$Proc.PrivateMemorySize64
}

function Get-PrivateWorkingSetMap {
    $map = @{}
    try {
        $rows = Get-CimInstance -ClassName Win32_PerfRawData_PerfProc_Process -ErrorAction Stop
        foreach ($r in $rows) {
            $id = [int]$r.IDProcess
            if ($id -gt 0) {
                $map[$id] = [double]$r.WorkingSetPrivate
            }
        }
    } catch {
        # Return empty map if perf counters are unavailable
    }
    return $map
}

function Get-ChildPidsMap {
    $children = @{}
    try {
        $procs = Get-CimInstance -ClassName Win32_Process -Property ProcessId, ParentProcessId -ErrorAction Stop
        foreach ($p in $procs) {
            $ppid = [int]$p.ParentProcessId
            $pidVal = [int]$p.ProcessId
            if (-not $children.ContainsKey($ppid)) {
                $children[$ppid] = New-Object System.Collections.Generic.List[int]
            }
            $children[$ppid].Add($pidVal)
        }
    } catch {
    }
    return $children
}

function Get-DescendantPids([int]$RootPid, $ChildrenMap) {
    $seen = New-Object System.Collections.Generic.HashSet[int]
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($RootPid)
    while ($queue.Count -gt 0) {
        $cur = $queue.Dequeue()
        if ($ChildrenMap.ContainsKey($cur)) {
            foreach ($child in $ChildrenMap[$cur]) {
                if ($seen.Add($child)) {
                    $queue.Enqueue($child)
                }
            }
        }
    }
    return $seen
}

$outDir = Split-Path -Parent $OutputCsv
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

# Record open apps at start of run (Section 8 protocol).
$openApps = Get-Process | Where-Object { $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object ProcessName, Id, MainWindowTitle
$logDir = if ($outDir) { $outDir } else { "." }
$appsLogPath = Join-Path $logDir "open-apps-$Condition.txt"
$openAppsText = ($openApps | Format-Table -AutoSize | Out-String)
$headerBanner = "Run started: $(Get-Date -Format 'o') | Condition: $Condition | Logical CPUs: $((Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors)"
Set-Content -Path $appsLogPath -Value ($headerBanner + "`r`n" + $openAppsText) -Encoding UTF8
Write-Host $headerBanner
Write-Host "Open apps recorded to $appsLogPath"

$startedByScript = $false
$mainProc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $mainProc -and $ExePath) {
    if (-not (Test-Path $ExePath)) {
        throw "Executable not found: $ExePath"
    }
    Write-Host "Launching $ExePath $($ExtraArgs -join ' ')"
    if ($ExtraArgs.Count -gt 0) {
        $mainProc = Start-Process -FilePath $ExePath -ArgumentList $ExtraArgs -PassThru
    } else {
        $mainProc = Start-Process -FilePath $ExePath -PassThru
    }
    $startedByScript = $true
    Start-Sleep -Seconds 5
}

if (-not $mainProc) {
    throw "Process '$ProcessName' is not running. Start it or pass -ExePath."
}

$csvHeader = "timestamp,condition,elapsed_sec,process_count,webview_count,main_pid,main_priv_commit_mb,main_priv_ws_mb,main_ws_mb,webview_priv_commit_mb,total_priv_commit_mb,total_priv_ws_mb,total_ws_mb,main_cpu_pct_1core,webview_cpu_pct_1core,total_cpu_pct_1core"
if (-not (Test-Path $OutputCsv)) {
    Set-Content -Path $OutputCsv -Value $csvHeader -Encoding UTF8
}

$sampleCount = [int][Math]::Floor($DurationSec / $IntervalSec) + 1
$prevTime = $null
$prevMainCpuSec = 0.0
$prevWebviewCpuSec = 0.0
$samples = New-Object System.Collections.Generic.List[object]

try {
    for ($i = 0; $i -lt $sampleCount; $i++) {
        $now = Get-Date
        $elapsedSec = $i * $IntervalSec

        $mainProc = Get-Process -Id $mainProc.Id -ErrorAction Stop
        $mainProc.Refresh()

        $childrenMap = Get-ChildPidsMap
        $descendants = Get-DescendantPids -RootPid $mainProc.Id -ChildrenMap $childrenMap
        $allProcs = Get-Process
        $webviewProcs = @($allProcs | Where-Object {
            $_.ProcessName -eq "msedgewebview2" -and ($descendants.Contains($_.Id) -or $ProcessName -ne "orbit-egui")
        })
        $relatedProcs = @($mainProc) + $webviewProcs

        $privWsMap = Get-PrivateWorkingSetMap

        $mainPrivCommitMb = (Get-ProcessPrivateCommitBytes $mainProc) / 1MB
        $mainWsMb = [double]$mainProc.WorkingSet64 / 1MB
        $mainPrivWsMb = if ($privWsMap.ContainsKey($mainProc.Id)) { $privWsMap[$mainProc.Id] / 1MB } else { 0.0 }

        $webviewPrivCommitMb = 0.0
        $webviewPrivWsMb = 0.0
        $webviewWsMb = 0.0
        $webviewCpuSec = 0.0
        foreach ($wp in $webviewProcs) {
            try {
                $wp.Refresh()
                $webviewPrivCommitMb += (Get-ProcessPrivateCommitBytes $wp) / 1MB
                $webviewWsMb += [double]$wp.WorkingSet64 / 1MB
                if ($privWsMap.ContainsKey($wp.Id)) {
                    $webviewPrivWsMb += $privWsMap[$wp.Id] / 1MB
                }
                $webviewCpuSec += $wp.TotalProcessorTime.TotalSeconds
            } catch {
            }
        }

        $mainCpuSec = $mainProc.TotalProcessorTime.TotalSeconds
        $mainCpuPct = 0.0
        $webviewCpuPct = 0.0
        if ($null -ne $prevTime) {
            $wallSec = ($now - $prevTime).TotalSeconds
            if ($wallSec -gt 0.0) {
                $mainCpuPct = [Math]::Max(0.0, (($mainCpuSec - $prevMainCpuSec) / $wallSec) * 100.0)
                $webviewCpuPct = [Math]::Max(0.0, (($webviewCpuSec - $prevWebviewCpuSec) / $wallSec) * 100.0)
            }
        }
        $prevTime = $now
        $prevMainCpuSec = $mainCpuSec
        $prevWebviewCpuSec = $webviewCpuSec

        $totalPrivCommitMb = $mainPrivCommitMb + $webviewPrivCommitMb
        $totalPrivWsMb = $mainPrivWsMb + $webviewPrivWsMb
        $totalWsMb = $mainWsMb + $webviewWsMb
        $totalCpuPct = $mainCpuPct + $webviewCpuPct

        $row = [PSCustomObject]@{
            timestamp = $now.ToString("o")
            condition = $Condition
            elapsed_sec = $elapsedSec
            process_count = $relatedProcs.Count
            webview_count = $webviewProcs.Count
            main_pid = $mainProc.Id
            main_priv_commit_mb = [Math]::Round($mainPrivCommitMb, 2)
            main_priv_ws_mb = [Math]::Round($mainPrivWsMb, 2)
            main_ws_mb = [Math]::Round($mainWsMb, 2)
            webview_priv_commit_mb = [Math]::Round($webviewPrivCommitMb, 2)
            total_priv_commit_mb = [Math]::Round($totalPrivCommitMb, 2)
            total_priv_ws_mb = [Math]::Round($totalPrivWsMb, 2)
            total_ws_mb = [Math]::Round($totalWsMb, 2)
            main_cpu_pct_1core = [Math]::Round($mainCpuPct, 2)
            webview_cpu_pct_1core = [Math]::Round($webviewCpuPct, 2)
            total_cpu_pct_1core = [Math]::Round($totalCpuPct, 2)
        }
        $samples.Add($row)

        $line = @(
            $row.timestamp, $row.condition, $row.elapsed_sec, $row.process_count,
            $row.webview_count, $row.main_pid, $row.main_priv_commit_mb, $row.main_priv_ws_mb,
            $row.main_ws_mb, $row.webview_priv_commit_mb, $row.total_priv_commit_mb,
            $row.total_priv_ws_mb, $row.total_ws_mb, $row.main_cpu_pct_1core,
            $row.webview_cpu_pct_1core, $row.total_cpu_pct_1core
        ) -join ","
        Add-Content -Path $OutputCsv -Value $line -Encoding UTF8

        $tag = if ($i -eq 0) { " (discarded warmup sample)" } else { "" }
        Write-Host ("[{0}] t={1}s procs={2} webview={3} commit={4:N2} MB priv_ws={5:N2} MB cpu={6:N2}%{7}" -f `
            $Condition, $elapsedSec, $row.process_count, $row.webview_count, `
            $row.total_priv_commit_mb, $row.total_priv_ws_mb, $row.total_cpu_pct_1core, $tag)

        if ($i -lt ($sampleCount - 1)) {
            Start-Sleep -Seconds $IntervalSec
        }
    }
} finally {
    if ($StopAfter -and $startedByScript -and $mainProc -and -not $mainProc.HasExited) {
        Stop-Process -Id $mainProc.Id -Force -ErrorAction SilentlyContinue
    }
}

$kept = @($samples | Where-Object { $_.elapsed_sec -gt 0 })
if ($kept.Count -gt 0) {
    $commitStats = $kept | Measure-Object -Property total_priv_commit_mb -Average -Minimum -Maximum
    $wsStats = $kept | Measure-Object -Property total_priv_ws_mb -Average -Minimum -Maximum
    $cpuStats = $kept | Measure-Object -Property total_cpu_pct_1core -Average -Minimum -Maximum
    Write-Host ""
    Write-Host "=== Condition $Condition Summary (n=$($kept.Count), first sample discarded) ==="
    Write-Host ("Private Commit (MB): mean={0:N2}, range={1:N2}..{2:N2}" -f $commitStats.Average, $commitStats.Minimum, $commitStats.Maximum)
    Write-Host ("Private WS     (MB): mean={0:N2}, range={1:N2}..{2:N2}" -f $wsStats.Average, $wsStats.Minimum, $wsStats.Maximum)
    Write-Host ("Total CPU  (1 core): mean={0:N2}%, range={1:N2}%..{2:N2}%" -f $cpuStats.Average, $cpuStats.Minimum, $cpuStats.Maximum)
}
