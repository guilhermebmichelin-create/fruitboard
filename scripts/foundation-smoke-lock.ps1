# Foundation Smoke exclusive host lock.
#
# One shared test identity (com.fruitboard.desktop.foundation-smoke) owns one
# host data directory. Concurrent installed validation runs must not share or
# replace each other's database, install, or uninstall registration. A unique
# per-run package/data identity was rejected: it would require rebuilding the
# NSIS package per run with a different Tauri identifier, churn registry
# state, and introduce a production data-directory override merely for tests.
# The smallest safe mechanism is this exclusive host-level lock with explicit
# ownership and bounded, actionable failure.
#
# Scope: test orchestration only. Normal Fruitboard data
# (com.fruitboard.desktop) and the native database owner-lock behavior are
# unchanged. No production data-directory override is introduced here.
#
# Usage (automated smoke and manual journey share this file):
#   . (Join-Path $PSScriptRoot "foundation-smoke-lock.ps1")
#   $lockPath = Get-FoundationSmokeLockPath
#   $owner = New-FoundationSmokeLockOwner -RunId $runId -Purpose "windows-foundation-smoke" -RunRoot $runRoot -InstallDirectory $installDirectory
#   Acquire-FoundationSmokeLock -LockPath $lockPath -Owner $owner -ArchiveParent $runRoot -SharedDataDirectory $syntheticDataDirectory
#   try { ... install/launch/archive/uninstall ... } finally { Release-FoundationSmokeLock -LockPath $lockPath -RunId $runId }
#
# Rules enforced by this helper:
# - A second run fails before launching, installing, archiving data, or
#   uninstalling another run's package. Acquire before any of those effects.
# - Stale ownership is decided through verifiable process state
#   (Get-Process on the recorded PID). Never delete storage/owner.lock to
#   bypass storage_busy, and never kill unrelated processes.
# - Prior evidence is preserved through reversible Move-Item archival with
#   checked absolute paths. No Remove-Item of databases or evidence.
# - Lock release removes only the caller's own lock (runId match). Foreign or
#   missing locks warn closed without deletion.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-FoundationSmokeLockPath {
    $localAppData = $env:LOCALAPPDATA
    if ([string]::IsNullOrWhiteSpace($localAppData)) {
        throw "The Foundation Smoke lock requires LOCALAPPDATA."
    }
    return (Join-Path $localAppData "com.fruitboard.desktop.foundation-smoke.lock.json")
}

function New-FoundationSmokeLockOwner {
    param(
        [Parameter(Mandatory = $true)][string]$RunId,
        [Parameter(Mandatory = $true)][string]$Purpose,
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$InstallDirectory
    )

    if ([string]::IsNullOrWhiteSpace($RunId)) {
        throw "The Foundation Smoke lock requires a run identifier."
    }
    if ([string]::IsNullOrWhiteSpace($Purpose)) {
        throw "The Foundation Smoke lock requires a purpose."
    }
    return [ordered]@{
        schemaVersion = 1
        runId = $RunId
        pid = $PID
        startedUtc = ([DateTime]::UtcNow.ToString("o"))
        purpose = $Purpose
        runRoot = $RunRoot
        installDirectory = $InstallDirectory
    }
}

function Test-FoundationSmokeOwnerAlive {
    param(
        [Parameter(Mandatory = $true)][int]$OwnerPid
    )

    try {
        $null = Get-Process -Id $OwnerPid -ErrorAction Stop
        return $true
    }
    catch {
        return $false
    }
}

function Get-FoundationSmokeLiveAppProcesses {
    # Reports installed test-app processes that may hold the shared database.
    # The caller must fail closed with an actionable message; never kills here.
    return @(Get-Process -Name "fruitboard-desktop" -ErrorAction SilentlyContinue)
}

function Move-PreviousFoundationSmokeData {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDirectory,
        [Parameter(Mandatory = $true)][string]$DestinationDirectory,
        [Parameter(Mandatory = $true)][string]$ExpectedParent,
        [Parameter(Mandatory = $true)][string]$DestinationParent
    )

    $sourceFull = [System.IO.Path]::GetFullPath($SourceDirectory)
    $expectedFull = [System.IO.Path]::GetFullPath($ExpectedParent).TrimEnd('\') + '\'
    if (-not $sourceFull.StartsWith($expectedFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A smoke-test path escaped its expected parent."
    }
    $destinationFull = [System.IO.Path]::GetFullPath($DestinationDirectory)
    $destinationParentFull = [System.IO.Path]::GetFullPath($DestinationParent).TrimEnd('\') + '\'
    if (-not $destinationFull.StartsWith($destinationParentFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A smoke-test path escaped its expected parent."
    }
    if (-not (Test-Path -LiteralPath $SourceDirectory -PathType Container)) {
        return $null
    }
    if (Test-Path -LiteralPath $DestinationDirectory) {
        throw "The smoke-test archive destination must not already exist."
    }

    $databaseFile = Join-Path $SourceDirectory "storage\fruitboard.db"
    $hashBefore = $null
    if (Test-Path -LiteralPath $databaseFile -PathType Leaf) {
        $hashBefore = (Get-FileHash -LiteralPath $databaseFile -Algorithm SHA256).Hash
    }
    Move-Item -LiteralPath $SourceDirectory -Destination $DestinationDirectory
    if ((Test-Path -LiteralPath $SourceDirectory) -or (-not (Test-Path -LiteralPath $DestinationDirectory -PathType Container))) {
        throw "The smoke-test archival did not complete."
    }
    if ($null -ne $hashBefore) {
        $movedDatabase = Join-Path $DestinationDirectory "storage\fruitboard.db"
        if (-not (Test-Path -LiteralPath $movedDatabase -PathType Leaf)) {
            throw "The smoke-test archival lost the database."
        }
        $hashAfter = (Get-FileHash -LiteralPath $movedDatabase -Algorithm SHA256).Hash
        if ($hashAfter -ne $hashBefore) {
            throw "The smoke-test archival changed the database."
        }
    }
    return $DestinationDirectory
}

function Acquire-FoundationSmokeLock {
    param(
        [Parameter(Mandatory = $true)][string]$LockPath,
        [Parameter(Mandatory = $true)][System.Collections.Specialized.OrderedDictionary]$Owner,
        [Parameter(Mandatory = $true)][string]$ArchiveParent,
        [Parameter(Mandatory = $true)][string]$SharedDataDirectory
    )

    $localAppDataFull = [System.IO.Path]::GetFullPath($env:LOCALAPPDATA).TrimEnd('\') + '\'
    $lockFull = [System.IO.Path]::GetFullPath($LockPath)
    if (-not $lockFull.StartsWith($localAppDataFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A smoke-test path escaped its expected parent."
    }

    try {
        $stream = [System.IO.File]::Open(
            $LockPath,
            [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write,
            [System.IO.FileShare]::None
        )
        try {
            $writer = New-Object System.IO.StreamWriter($stream, [System.Text.Encoding]::UTF8)
            try {
                $writer.Write(($Owner | ConvertTo-Json -Depth 4))
                $writer.Flush()
            }
            finally {
                $writer.Dispose()
            }
        }
        finally {
            $stream.Dispose()
        }
        return $Owner
    }
    catch [System.IO.IOException] {
        $existingRaw = Get-Content -LiteralPath $LockPath -Raw -ErrorAction Stop
        $existing = $null
        try {
            $existing = $existingRaw | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            throw "Another Foundation Smoke run owns the shared test identity but its lock is unreadable. Coordinate an exclusive window before retrying; do not delete owner.lock or kill unrelated processes. Lock: com.fruitboard.desktop.foundation-smoke.lock.json."
        }
        $ownerPid = 0
        if (-not [int]::TryParse([string]$existing.pid, [ref]$ownerPid)) {
            throw "Another Foundation Smoke run owns the shared test identity but its lock is unreadable. Coordinate an exclusive window before retrying; do not delete owner.lock or kill unrelated processes. Lock: com.fruitboard.desktop.foundation-smoke.lock.json."
        }
        if (Test-FoundationSmokeOwnerAlive -OwnerPid $ownerPid) {
            $ownerRun = [string]$existing.runId
            $ownerStarted = [string]$existing.startedUtc
            throw "Another Foundation Smoke run owns the shared test identity (runId $ownerRun, pid $ownerPid, started $ownerStarted). Wait for its lock release or coordinate an exclusive window before retrying; do not delete owner.lock or kill unrelated processes. Lock: com.fruitboard.desktop.foundation-smoke.lock.json."
        }

        $live = @(Get-FoundationSmokeLiveAppProcesses)
        if ($live.Count -gt 0) {
            throw "A stale Foundation Smoke lock remains but installed app processes are still running. Close every fruitboard-desktop process gracefully before archival, then coordinate an exclusive window; do not delete owner.lock or kill unrelated processes. Lock: com.fruitboard.desktop.foundation-smoke.lock.json."
        }

        $staleRun = [string]$existing.runId
        if ([string]::IsNullOrWhiteSpace($staleRun)) {
            $staleRun = "unreadable"
        }
        $archiveDirectory = Join-Path $ArchiveParent "archived-previous-foundation-smoke-$staleRun"
        if (Test-Path -LiteralPath $SharedDataDirectory -PathType Container) {
            $null = Move-PreviousFoundationSmokeData -SourceDirectory $SharedDataDirectory -DestinationDirectory $archiveDirectory -ExpectedParent $env:LOCALAPPDATA -DestinationParent $ArchiveParent
        }
        $staleLockArchive = Join-Path $ArchiveParent "archived-stale-smoke-lock-$staleRun.json"
        if (-not (Test-Path -LiteralPath $staleLockArchive)) {
            Move-Item -LiteralPath $LockPath -Destination $staleLockArchive
        }
        else {
            throw "The stale smoke-lock archive destination must not already exist."
        }

        try {
            $retryStream = [System.IO.File]::Open(
                $LockPath,
                [System.IO.FileMode]::CreateNew,
                [System.IO.FileAccess]::Write,
                [System.IO.FileShare]::None
            )
            try {
                $retryWriter = New-Object System.IO.StreamWriter($retryStream, [System.Text.Encoding]::UTF8)
                try {
                    $retryWriter.Write(($Owner | ConvertTo-Json -Depth 4))
                    $retryWriter.Flush()
                }
                finally {
                    $retryWriter.Dispose()
                }
            }
            finally {
                $retryStream.Dispose()
            }
            return $Owner
        }
        catch [System.IO.IOException] {
            throw "Another Foundation Smoke run acquired the shared test identity during stale recovery. Coordinate an exclusive window before retrying; do not delete owner.lock or kill unrelated processes. Lock: com.fruitboard.desktop.foundation-smoke.lock.json."
        }
    }
}

function Release-FoundationSmokeLock {
    param(
        [Parameter(Mandatory = $true)][string]$LockPath,
        [Parameter(Mandatory = $true)][string]$RunId
    )

    if (-not (Test-Path -LiteralPath $LockPath -PathType Leaf)) {
        Write-Warning "The Foundation Smoke lock is already released."
        return
    }
    try {
        $existing = Get-Content -LiteralPath $LockPath -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
    }
    catch {
        Write-Warning "The Foundation Smoke lock is unreadable; preserving it for inspection."
        return
    }
    if ([string]$existing.runId -ne $RunId) {
        Write-Warning "The Foundation Smoke lock belongs to another run; preserving it for inspection."
        return
    }
    Remove-Item -LiteralPath $LockPath -Force
}
