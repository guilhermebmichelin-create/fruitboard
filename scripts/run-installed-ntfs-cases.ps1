[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$JourneyRoot,
    [string]$InstallerPath,
    [string]$NodePath,
    [string]$PythonPath,
    [string]$UvPath,
    [string]$RustBinDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "The installed NTFS evidence run is Windows-only."
}
if ([string]::IsNullOrWhiteSpace($env:TEMP)) {
    throw "TEMP is required so the disposable journey root cannot be placed in a repository or user-data directory."
}

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$journeyRootFull = [System.IO.Path]::GetFullPath($JourneyRoot)
$tempRootFull = [System.IO.Path]::GetFullPath($env:TEMP).TrimEnd("\") + "\"
if (-not $journeyRootFull.StartsWith($tempRootFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "JourneyRoot must be a new disposable directory beneath TEMP."
}
if (Test-Path -LiteralPath $journeyRootFull) {
    throw "JourneyRoot already exists; refusing to overwrite prior evidence."
}

if ([string]::IsNullOrWhiteSpace($NodePath)) {
    $NodePath = (Get-Command node.exe -ErrorAction Stop).Source
}
if (-not (Test-Path -LiteralPath $NodePath -PathType Leaf)) {
    throw "NodePath is not an executable file."
}
if ([string]::IsNullOrWhiteSpace($PythonPath)) {
    $PythonPath = (Get-Command python.exe -ErrorAction Stop).Source
}
if (-not (Test-Path -LiteralPath $PythonPath -PathType Leaf)) {
    throw "PythonPath is not an executable file."
}
if ([string]::IsNullOrWhiteSpace($UvPath)) {
    $UvPath = (Get-Command uv.exe -ErrorAction Stop).Source
}
if (-not (Test-Path -LiteralPath $UvPath -PathType Leaf)) {
    throw "UvPath is not an executable file."
}
if ([string]::IsNullOrWhiteSpace($RustBinDirectory)) {
    $RustBinDirectory = Split-Path -Parent (Get-Command rustc.exe -ErrorAction Stop).Source
}
$rustcPath = Join-Path $RustBinDirectory "rustc.exe"
$cargoPath = Join-Path $RustBinDirectory "cargo.exe"
$rustupPath = Join-Path $RustBinDirectory "rustup.exe"
foreach ($requiredTool in @($rustcPath, $cargoPath, $rustupPath)) {
    if (-not (Test-Path -LiteralPath $requiredTool -PathType Leaf)) {
        throw "RustBinDirectory is missing a required tool: $requiredTool"
    }
}

$pnpmCommand = (Get-Command pnpm.cmd -ErrorAction Stop).Source
$pnpmScript = Join-Path (Split-Path -Parent $pnpmCommand) "node_modules\pnpm\bin\pnpm.mjs"
if (-not (Test-Path -LiteralPath $pnpmScript -PathType Leaf)) {
    throw "The pinned pnpm script is missing beside pnpm.cmd: $pnpmScript"
}

$driverPath = Join-Path $repositoryRoot "docs\review\phase-2-integration\installed-journey\installed-ntfs-cases.mjs"
$inspectorPath = Join-Path $repositoryRoot "scripts\inspect-foundation-smoke-db.py"
$lockHelperPath = Join-Path $PSScriptRoot "foundation-smoke-lock.ps1"
foreach ($requiredPath in @($driverPath, $inspectorPath, $lockHelperPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Required orchestration file is missing: $requiredPath"
    }
}

. $lockHelperPath

$runId = [Guid]::NewGuid().ToString("N")
$installDirectory = Join-Path $journeyRootFull "Installed Fruitboard"
$sharedDataDirectory = Join-Path $env:LOCALAPPDATA "com.fruitboard.desktop.foundation-smoke"
$databasePath = Join-Path $sharedDataDirectory "storage\fruitboard.db"
$lockPath = Get-FoundationSmokeLockPath
$lockOwner = New-FoundationSmokeLockOwner `
    -RunId $runId `
    -Purpose "installed-ntfs-cases-20260912" `
    -RunRoot $journeyRootFull `
    -InstallDirectory $installDirectory
$lockAcquired = $false
$packageInstalled = $false
$installerFull = $null
$driverExitCode = $null
$aclIdentity = $null
$cleanupFailure = $null
$live = @()
$worktreeVenvPath = Join-Path $repositoryRoot ".venv"
$createdPythonEnvironmentLink = $false
$buildMode = if ([string]::IsNullOrWhiteSpace($InstallerPath)) { "reproducible-local-build" } else { "provided-recorded-installer" }
$oldPath = $env:PATH
$oldNpmExecPath = $env:npm_execpath
$nodeDirectory = Split-Path -Parent $NodePath
$uvDirectory = Split-Path -Parent $UvPath
$env:PATH = $nodeDirectory + ";" + $RustBinDirectory + ";" + $uvDirectory + ";" + $env:PATH
$env:npm_execpath = $pnpmScript

function Assert-ContainedPath {
    param(
        [Parameter(Mandatory = $true)][string]$Candidate,
        [Parameter(Mandatory = $true)][string]$Parent
    )

    $candidateFull = [System.IO.Path]::GetFullPath($Candidate)
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd("\") + "\"
    if (-not $candidateFull.StartsWith($parentFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A disposable evidence path escaped its expected parent."
    }
}

function Assert-LockOwner {
    $owner = Get-Content -LiteralPath $lockPath -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
    if ([int]$owner.schemaVersion -ne 1 -or [string]$owner.runId -ne $runId -or [int]$owner.pid -ne $PID) {
        throw "The orchestration lock is not owned by this run."
    }
    if (-not (Test-FoundationSmokeOwnerAlive -OwnerPid $PID)) {
        throw "The orchestration lock owner is no longer alive."
    }
}

function Invoke-CapturedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $FilePath @ArgumentList 2>&1 | ForEach-Object { $_.ToString() } | Out-String
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldErrorActionPreference
    }
    $output | Set-Content -LiteralPath $OutputPath -Encoding utf8
    if ($exitCode -ne 0) {
        throw "$FilePath failed with exit code $exitCode. See $OutputPath."
    }
    return $output
}

function Get-ArtifactRecord {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Expected artifact is missing: $Path"
    }
    $item = Get-Item -LiteralPath $Path
    return [ordered]@{
        name = $item.Name
        bytes = [long]$item.Length
        sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    }
}

function Install-Package {
    param([Parameter(Mandatory = $true)][string]$Path)

    $process = Start-Process -FilePath $Path -ArgumentList @("/S", "/D=$installDirectory") -PassThru -Wait -WindowStyle Hidden
    if ($process.ExitCode -ne 0) {
        throw "The NSIS installer failed with exit code $($process.ExitCode)."
    }
    if (-not (Test-Path -LiteralPath $installDirectory -PathType Container)) {
        throw "The NSIS installer did not create the run-specific install directory."
    }
}

function Uninstall-Package {
    $uninstaller = Join-Path $installDirectory "uninstall.exe"
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        throw "The run-specific NSIS uninstaller is missing."
    }
    $process = Start-Process -FilePath $uninstaller -ArgumentList @("/S") -PassThru -Wait -WindowStyle Hidden
    if ($process.ExitCode -ne 0) {
        throw "The NSIS uninstaller failed with exit code $($process.ExitCode)."
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((Test-Path -LiteralPath $installDirectory) -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 100
    }
    if (Test-Path -LiteralPath $installDirectory) {
        throw "The NSIS uninstaller did not remove the run-specific install directory."
    }
}

function Restore-DisposableAcl {
    $blockedDirectory = Join-Path $journeyRootFull "fixtures\denied\scan-root\blocked"
    if (-not (Test-Path -LiteralPath $blockedDirectory -PathType Container)) {
        return
    }
    if ([string]::IsNullOrWhiteSpace($script:aclIdentity)) {
        $script:aclIdentity = ((& whoami.exe) | Out-String).Trim().Split("`n")[0].Trim()
    }
    if ([string]::IsNullOrWhiteSpace($script:aclIdentity)) {
        throw "Cannot restore the disposable ACL because whoami returned no identity."
    }
    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & icacls.exe $blockedDirectory /remove:d $script:aclIdentity /T /C 2>&1 | ForEach-Object { $_.ToString() } | Out-String
        $aclExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldErrorActionPreference
    }
    if ($aclExitCode -ne 0) {
        throw "Disposable ACL restoration failed; preserve the journey root and inspect it."
    }
    try {
        Get-ChildItem -LiteralPath $blockedDirectory -Force -ErrorAction Stop | Out-Null
    }
    catch {
        throw "Disposable ACL restoration did not restore same-user traversal."
    }
    $output | Set-Content -LiteralPath (Join-Path $journeyRootFull "acl-restore-wrapper.txt") -Encoding utf8
}

function Invoke-DatabaseInspector {
    param([Parameter(Mandatory = $true)][string]$SnapshotDirectory)

    $snapshotDatabase = Join-Path $SnapshotDirectory "fruitboard.db"
    if (-not (Test-Path -LiteralPath $snapshotDatabase -PathType Leaf)) {
        return
    }
    Assert-LockOwner
    $outputPath = Join-Path $SnapshotDirectory "database-evidence.json"
    $arguments = @(
        $inspectorPath,
        "--database", $snapshotDatabase,
        "--journey-root", $journeyRootFull,
        "--output", $outputPath
    )
    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $PythonPath @arguments 2>&1 | ForEach-Object { $_.ToString() } | Out-String
        $inspectorExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $oldErrorActionPreference
    }
    if ($inspectorExitCode -ne 0) {
        throw "The read-only database inspector failed for $SnapshotDirectory."
    }
    $output | Set-Content -LiteralPath (Join-Path $SnapshotDirectory "database-inspector.stdout.txt") -Encoding utf8
}

function Ensure-WorktreePythonEnvironment {
    if (Test-Path -LiteralPath (Join-Path $worktreeVenvPath "Scripts\python.exe") -PathType Leaf) {
        return
    }
    $providedEnvironment = Split-Path -Parent (Split-Path -Parent $PythonPath)
    if (-not (Test-Path -LiteralPath (Join-Path $providedEnvironment "pyvenv.cfg") -PathType Leaf)) {
        throw "The worktree .venv is missing and PythonPath does not identify a reusable pinned venv."
    }
    if (Test-Path -LiteralPath $worktreeVenvPath) {
        throw "The worktree .venv exists but does not contain the pinned Windows Python executable."
    }
    New-Item -ItemType Junction -Path $worktreeVenvPath -Target $providedEnvironment | Out-Null
    $script:createdPythonEnvironmentLink = $true
}

Assert-ContainedPath -Candidate $journeyRootFull -Parent $env:TEMP
Assert-ContainedPath -Candidate $installDirectory -Parent $journeyRootFull
New-Item -ItemType Directory -Path $journeyRootFull | Out-Null

try {
    Ensure-WorktreePythonEnvironment
    # Fail before touching shared data or starting a package if another run is
    # active. The lock helper also preserves stale evidence reversibly.
    if (@(Get-FoundationSmokeLiveAppProcesses).Count -gt 0) {
        throw "An installed Fruitboard process is already running; coordinate the exclusive window first."
    }
    $null = Acquire-FoundationSmokeLock `
        -LockPath $lockPath `
        -Owner $lockOwner `
        -ArchiveParent $journeyRootFull `
        -SharedDataDirectory $sharedDataDirectory
    $lockAcquired = $true
    Assert-LockOwner
    if (@(Get-FoundationSmokeLiveAppProcesses).Count -gt 0) {
        throw "An installed Fruitboard process appeared during lock acquisition; refusing to touch shared data."
    }

    # Archive the prior dedicated smoke database only after ownership is
    # established. Move-PreviousFoundationSmokeData verifies its database
    # hash before and after the move and never deletes prior evidence.
    $previousArchive = Join-Path $journeyRootFull "archived-previous-foundation-smoke-$runId"
    $archivedPreviousData = $false
    if (Test-Path -LiteralPath $sharedDataDirectory -PathType Container) {
        $null = Move-PreviousFoundationSmokeData `
            -SourceDirectory $sharedDataDirectory `
            -DestinationDirectory $previousArchive `
            -ExpectedParent $env:LOCALAPPDATA `
            -DestinationParent $journeyRootFull
        $archivedPreviousData = $true
    }

    $buildStartedUtc = [DateTime]::UtcNow.ToString("o")
    $toolchainOutput = Join-Path $journeyRootFull "toolchain-verification.stdout.txt"
    Push-Location $repositoryRoot
    try {
        $null = Invoke-CapturedProcess `
            -FilePath $NodePath `
            -ArgumentList @($pnpmScript, "verify:toolchains") `
            -OutputPath $toolchainOutput
    }
    finally {
        Pop-Location
    }
    if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
        $prepareOutput = Join-Path $journeyRootFull "build-prepare.stdout.txt"
        $null = Invoke-CapturedProcess `
            -FilePath $NodePath `
            -ArgumentList @((Join-Path $repositoryRoot "scripts\prepare-foundation-sidecar.mjs")) `
            -OutputPath $prepareOutput
        $buildOutput = Join-Path $journeyRootFull "build.stdout.txt"
        Push-Location $repositoryRoot
        try {
            $null = Invoke-CapturedProcess `
                -FilePath $NodePath `
                -ArgumentList @(
                    $pnpmScript,
                    "--filter", "@fruitboard/desktop", "exec", "tauri", "build", "--ci", "--no-sign",
                    "--config", "src-tauri/tauri.package.conf.json", "--features", "packaging-smoke,scan-console", "--bundles", "nsis"
                ) `
                -OutputPath $buildOutput
        }
        finally {
            Pop-Location
        }
        $buildStarted = [DateTime]::Parse($buildStartedUtc).ToUniversalTime()
        $bundleDirectory = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot "target\release\bundle\nsis")).Path
        $built = Get-ChildItem -LiteralPath $bundleDirectory -Filter "*.exe" -File |
            Where-Object { $_.LastWriteTimeUtc -ge $buildStarted } |
            Sort-Object LastWriteTimeUtc -Descending |
            Select-Object -First 1
        if ($null -eq $built) {
            throw "The feature-enabled NSIS build produced no new installer."
        }
        $installerFull = $built.FullName
    }
    else {
        $installerFull = (Resolve-Path -LiteralPath $InstallerPath).Path
    }

    $bundleDirectory = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot "target\release\bundle\nsis")).Path.TrimEnd("\") + "\"
    if (-not $installerFull.StartsWith($bundleDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "InstallerPath must point inside this worktree's target\release\bundle\nsis directory."
    }
    if (-not (Test-Path -LiteralPath $installerFull -PathType Leaf)) {
        throw "InstallerPath is not a file."
    }
    $installerCopy = Join-Path $journeyRootFull "package\Fruitboard Foundation Smoke setup.exe"
    New-Item -ItemType Directory -Path (Split-Path -Parent $installerCopy) | Out-Null
    Copy-Item -LiteralPath $installerFull -Destination $installerCopy

    $toolVersions = [ordered]@{
        node = (& $NodePath --version | Out-String).Trim()
        pnpm = (& $NodePath $pnpmScript --version | Out-String).Trim()
        rustc = (& $rustcPath --version | Out-String).Trim()
        cargo = (& $cargoPath --version | Out-String).Trim()
        uv = (& $UvPath --version | Out-String).Trim()
        python = (& $PythonPath --version 2>&1 | Out-String).Trim()
        windows = [Environment]::OSVersion.Version.ToString()
    }
    $provenance = [ordered]@{
        schemaVersion = 1
        runId = $runId
        commit = ((git -C $repositoryRoot rev-parse HEAD) | Out-String).Trim()
        originMain = ((git -C $repositoryRoot rev-parse origin/main) | Out-String).Trim()
        historicalS5Commit = "19585dae7bef9ffdec06b71e0627df1b3f7ceb2f"
        packageFeatures = @("packaging-smoke", "scan-console")
        buildMode = $buildMode
        buildStartedUtc = $buildStartedUtc
        buildCommand = if ($buildMode -eq "reproducible-local-build") { "node scripts/prepare-foundation-sidecar.mjs; pnpm.cmd --filter @fruitboard/desktop exec tauri build --ci --no-sign --config src-tauri/tauri.package.conf.json --features packaging-smoke,scan-console --bundles nsis" } else { "provided exact installer; build performed previously and installer hash recorded" }
        toolVersions = $toolVersions
        toolPaths = [ordered]@{
            node = $NodePath
            pnpm = $pnpmScript
            corepack = (Join-Path $repositoryRoot "node_modules\corepack\dist\corepack.js")
            rustc = $rustcPath
            cargo = $cargoPath
            rustup = $rustupPath
            uv = $UvPath
            python = $PythonPath
        }
        lock = [ordered]@{
            file = "com.fruitboard.desktop.foundation-smoke.lock.json"
            runId = $runId
            ownerPid = $PID
            purpose = $lockOwner.purpose
            archivedPreviousData = $archivedPreviousData
        }
        artifacts = [ordered]@{
            lockHelper = Get-ArtifactRecord -Path $lockHelperPath
            wrapper = Get-ArtifactRecord -Path (Join-Path $PSScriptRoot "run-installed-ntfs-cases.ps1")
            driver = Get-ArtifactRecord -Path $driverPath
            databaseInspector = Get-ArtifactRecord -Path $inspectorPath
            node = Get-ArtifactRecord -Path $NodePath
            pnpm = Get-ArtifactRecord -Path $pnpmScript
            corepack = Get-ArtifactRecord -Path (Join-Path $repositoryRoot "node_modules\corepack\dist\corepack.js")
            rustc = Get-ArtifactRecord -Path $rustcPath
            cargo = Get-ArtifactRecord -Path $cargoPath
            rustup = Get-ArtifactRecord -Path $rustupPath
            uv = Get-ArtifactRecord -Path $UvPath
            python = Get-ArtifactRecord -Path $PythonPath
            cargoLock = Get-ArtifactRecord -Path (Join-Path $repositoryRoot "Cargo.lock")
            pnpmLock = Get-ArtifactRecord -Path (Join-Path $repositoryRoot "pnpm-lock.yaml")
            installer = Get-ArtifactRecord -Path $installerCopy
        }
    }
    $provenance | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $journeyRootFull "provenance.json") -Encoding utf8

    Install-Package -Path $installerCopy
    $packageInstalled = $true
    $applicationPath = Join-Path $installDirectory "fruitboard-desktop.exe"
    if (-not (Test-Path -LiteralPath $applicationPath -PathType Leaf)) {
        throw "The installed application executable is missing."
    }
    $installedArtifacts = [ordered]@{
        application = Get-ArtifactRecord -Path $applicationPath
        sidecar = if (Test-Path -LiteralPath (Join-Path $installDirectory "fruitboard-sidecar-smoke.exe") -PathType Leaf) {
            Get-ArtifactRecord -Path (Join-Path $installDirectory "fruitboard-sidecar-smoke.exe")
        } else { $null }
    }
    $installedArtifacts | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $journeyRootFull "installed-artifacts.json") -Encoding utf8

    Assert-LockOwner
    $oldLockPath = $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PATH
    $oldLockRunId = $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_RUN_ID
    $oldLockPid = $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PID
    $oldDataDirectory = $env:FRUITBOARD_FOUNDATION_SMOKE_DATA_DIRECTORY
    $oldRepositoryRoot = $env:FRUITBOARD_REPOSITORY_ROOT
    try {
        $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PATH = $lockPath
        $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_RUN_ID = $runId
        $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PID = [string]$PID
        $env:FRUITBOARD_FOUNDATION_SMOKE_DATA_DIRECTORY = $sharedDataDirectory
        $env:FRUITBOARD_REPOSITORY_ROOT = $repositoryRoot
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $driverOutput = & $NodePath $driverPath $journeyRootFull $applicationPath $lockPath $runId 2>&1 | ForEach-Object { $_.ToString() } | Tee-Object -FilePath (Join-Path $journeyRootFull "driver.stdout.txt")
            $driverExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $oldErrorActionPreference
        }
    }
    finally {
        if ($null -eq $oldLockPath) { Remove-Item Env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PATH -ErrorAction SilentlyContinue } else { $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PATH = $oldLockPath }
        if ($null -eq $oldLockRunId) { Remove-Item Env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_RUN_ID -ErrorAction SilentlyContinue } else { $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_RUN_ID = $oldLockRunId }
        if ($null -eq $oldLockPid) { Remove-Item Env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PID -ErrorAction SilentlyContinue } else { $env:FRUITBOARD_FOUNDATION_SMOKE_LOCK_PID = $oldLockPid }
        if ($null -eq $oldDataDirectory) { Remove-Item Env:FRUITBOARD_FOUNDATION_SMOKE_DATA_DIRECTORY -ErrorAction SilentlyContinue } else { $env:FRUITBOARD_FOUNDATION_SMOKE_DATA_DIRECTORY = $oldDataDirectory }
        if ($null -eq $oldRepositoryRoot) { Remove-Item Env:FRUITBOARD_REPOSITORY_ROOT -ErrorAction SilentlyContinue } else { $env:FRUITBOARD_REPOSITORY_ROOT = $oldRepositoryRoot }
    }
    foreach ($snapshot in Get-ChildItem -LiteralPath $journeyRootFull -Directory -Filter "database-*" | Sort-Object Name) {
        Invoke-DatabaseInspector -SnapshotDirectory $snapshot.FullName
    }
    if (-not (Test-Path -LiteralPath $databasePath -PathType Leaf)) {
        throw "The installed run did not leave the dedicated database available for evidence."
    }
    if ($driverExitCode -ne 0) {
        throw "The installed NTFS driver failed with exit code $driverExitCode; all evidence remains under JourneyRoot."
    }
}
finally {
    # The driver stops its own exact app PID. Never kill an unrelated process;
    # if one remains, leave the run root and lock for owner coordination.
    try {
        try {
            Restore-DisposableAcl
        }
        catch {
            $cleanupFailure = $_.Exception.Message
            Write-Warning $_.Exception.Message
        }
        if ($packageInstalled) {
            $live = @(Get-FoundationSmokeLiveAppProcesses)
            if ($live.Count -eq 0) {
                try {
                    Uninstall-Package
                }
                catch {
                    $cleanupFailure = $_.Exception.Message
                    Write-Warning $_.Exception.Message
                }
            }
            else {
                $cleanupFailure = "An installed Fruitboard process remains; the package was not uninstalled."
                Write-Warning "An installed Fruitboard process remains; preserving the package and refusing to uninstall around a live process."
            }
            if ($live.Count -gt 0) {
                Write-Warning "The orchestration lock is being preserved because an installed Fruitboard process remains."
            }
        }
        if ($lockAcquired -and $live.Count -eq 0) {
            Release-FoundationSmokeLock -LockPath $lockPath -RunId $runId
            $lockAcquired = $false
        }
    }
    finally {
        if ($null -eq $oldNpmExecPath) { Remove-Item Env:npm_execpath -ErrorAction SilentlyContinue } else { $env:npm_execpath = $oldNpmExecPath }
        $env:PATH = $oldPath
    }
    if ($createdPythonEnvironmentLink -and (Test-Path -LiteralPath $worktreeVenvPath)) {
        $venvItem = Get-Item -LiteralPath $worktreeVenvPath -Force
        if (($venvItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Remove-Item -LiteralPath $worktreeVenvPath -Force
        }
    }
    if ($null -ne $cleanupFailure) {
        throw $cleanupFailure
    }
}
