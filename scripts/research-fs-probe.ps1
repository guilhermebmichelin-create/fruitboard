#Requires -Version 5.1
<#
.SYNOPSIS
    Reproducible filesystem probes behind spike findings #42/#43.

.DESCRIPTION
    Runs the disposable-tree experiments from docs/research/p0-e-file-identity.md
    (Identity mode) and docs/research/p0-d-drivefs-watcher.md (Watcher mode)
    with assertions. All output is path-free counts and booleans; synthetic
    trees are created under the system temp directory and removed afterwards.
    Requires Windows (fsutil plus .NET FileSystemWatcher).

.EXAMPLE
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/research-fs-probe.ps1
#>
[CmdletBinding()]
param(
    [ValidateSet("Identity", "Watcher", "All")]
    [string]$Mode = "All"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "The research filesystem probe requires Windows."
}

$script:failures = New-Object Collections.ArrayList

function Assert-Probe {
    param([bool]$Condition, [string]$Name)

    if ($Condition) {
        Write-Output "PASS: $Name"
    } else {
        Write-Output "FAIL: $Name"
        $script:failures.Add($Name) | Out-Null
    }
}

function Get-FileIdentity($LiteralPath) {
    $line = fsutil file queryfileid "$LiteralPath" 2>&1 | Select-Object -First 1
    if ($line -notmatch "0x[0-9a-fA-F]{32}") {
        throw "The filesystem did not report a usable file identity."
    }
    return $Matches[0].ToLowerInvariant()
}

function Get-VolumeSerial($LiteralPath) {
    $root = [System.IO.Path]::GetPathRoot((Resolve-Path -LiteralPath $LiteralPath).Path)
    $drive = $root.TrimEnd("\")
    $disk = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='$drive'"
    if ($null -eq $disk) {
        throw "The test volume serial is not observable."
    }
    return $disk.VolumeSerialNumber
}

function Invoke-IdentityProbe {
    $root = Join-Path $env:TEMP ("fruitboard-research-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $root | Out-Null
    try {
        $serialBefore = Get-VolumeSerial $root
        $file = Join-Path $root "original.bin"
        [IO.File]::WriteAllBytes($file, (New-Object byte[] 1024))
        $created = Get-FileIdentity $file

        $renamed = Join-Path $root "renamed.bin"
        Rename-Item -LiteralPath $file -NewName "renamed.bin"
        Assert-Probe ((Get-FileIdentity $renamed) -eq $created) "rename preserves identity"

        $subdirectory = Join-Path $root "sub"
        New-Item -ItemType Directory -Path $subdirectory | Out-Null
        Move-Item -LiteralPath $renamed -Destination $subdirectory
        $moved = Join-Path $subdirectory "renamed.bin"
        Assert-Probe ((Get-FileIdentity $moved) -eq $created) "same-volume move preserves identity"

        [IO.File]::WriteAllBytes($moved, (New-Object byte[] 4096))
        Assert-Probe ((Get-FileIdentity $moved) -eq $created) "in-place overwrite preserves identity"

        $copy = Join-Path $root "copy.bin"
        Copy-Item -LiteralPath $moved -Destination $copy
        Assert-Probe ((Get-FileIdentity $copy) -ne $created) "copy mints a new identity"

        $replacement = Join-Path $root "replacement.bin"
        [IO.File]::WriteAllBytes($replacement, (New-Object byte[] 512))
        Move-Item -LiteralPath $replacement -Destination $moved -Force
        $replaced = Get-FileIdentity $moved
        Assert-Probe ($replaced -ne $created) "replacement save mints a new identity"

        $alias = Join-Path $root "alias.bin"
        New-Item -ItemType HardLink -Path $alias -Target $moved | Out-Null
        Assert-Probe ((Get-FileIdentity $alias) -eq $replaced) "hardlink alias shares identity"
        Remove-Item -LiteralPath $moved -Force
        Assert-Probe (Test-Path -LiteralPath $alias) "removing one alias keeps the other present"
        Assert-Probe ((Get-FileIdentity $alias) -eq $replaced) "surviving alias keeps identity"

        Remove-Item -LiteralPath $alias -Force
        [IO.File]::WriteAllBytes($moved, (New-Object byte[] 64))
        Assert-Probe ((Get-FileIdentity $moved) -ne $replaced) "delete plus recreate mints a new identity"

        Assert-Probe ((Get-VolumeSerial $root) -eq $serialBefore) "volume serial is stable"
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-WatcherProbe {
    $root = Join-Path $env:TEMP ("fruitboard-research-watch-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $root | Out-Null
    try {
        $watcher = New-Object IO.FileSystemWatcher $root
        $watcher.IncludeSubdirectories = $true
        $watcher.EnableRaisingEvents = $true
        # Queue events without -Action handlers: action scriptblocks cannot see
        # function-scope variables under StrictMode, so drain the session queue
        # explicitly after the operations complete.
        $subscriptions = @(
            Register-ObjectEvent $watcher Created -SourceIdentifier "ResearchCreated"
            Register-ObjectEvent $watcher Changed -SourceIdentifier "ResearchChanged"
            Register-ObjectEvent $watcher Renamed -SourceIdentifier "ResearchRenamed"
            Register-ObjectEvent $watcher Deleted -SourceIdentifier "ResearchDeleted"
            Register-ObjectEvent $watcher Error -SourceIdentifier "ResearchError"
        )
        try {
            1..50 | ForEach-Object {
                [IO.File]::WriteAllBytes((Join-Path $root ("burst$_.bin")), (New-Object byte[] 256))
            }
            Start-Sleep -Seconds 3
            Rename-Item -LiteralPath (Join-Path $root "burst1.bin") -NewName "burst1-renamed.bin"
            Remove-Item -LiteralPath (Join-Path $root "burst2.bin")
            1..10 | ForEach-Object {
                [IO.File]::AppendAllText((Join-Path $root "burst3.bin"), "x")
            }
            Start-Sleep -Seconds 3
        }
        finally {
            foreach ($subscription in $subscriptions) {
                Unregister-Event -SubscriptionId $subscription.Id -ErrorAction SilentlyContinue
            }
            $watcher.Dispose()
        }

        $created = @(Get-Event -SourceIdentifier "ResearchCreated" -ErrorAction SilentlyContinue).Count
        $changed = @(Get-Event -SourceIdentifier "ResearchChanged" -ErrorAction SilentlyContinue).Count
        $renamed = @(Get-Event -SourceIdentifier "ResearchRenamed" -ErrorAction SilentlyContinue).Count
        $deleted = @(Get-Event -SourceIdentifier "ResearchDeleted" -ErrorAction SilentlyContinue).Count
        $errors = @(Get-Event -SourceIdentifier "ResearchError" -ErrorAction SilentlyContinue).Count
        Remove-Event -SourceIdentifier "Research*" -ErrorAction SilentlyContinue
        Write-Output "OBSERVED created=$created changed=$changed renamed=$renamed deleted=$deleted errors=$errors"
        Assert-Probe ($created -eq 50) "burst creations are all reported"
        Assert-Probe ($renamed -eq 1) "rename arrives as one event"
        Assert-Probe ($deleted -eq 1) "delete arrives as one event"
        Assert-Probe ($changed -ge 10) "repeated appends are reported without coalescing"
    }
    finally {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($Mode -eq "Identity" -or $Mode -eq "All") {
    Invoke-IdentityProbe
}
if ($Mode -eq "Watcher" -or $Mode -eq "All") {
    Invoke-WatcherProbe
}

if ($script:failures.Count -gt 0) {
    throw "The research filesystem probe reported $($script:failures.Count) failure(s)."
}
Write-Output "RESEARCH PROBE: all assertions passed"
