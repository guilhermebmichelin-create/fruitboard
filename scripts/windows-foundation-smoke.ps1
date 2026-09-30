[CmdletBinding()]
param(
    [string]$EvidencePath = "docs/review/issue-18/windows-smoke.json",
    [string]$RetainedEvidenceRoot = "",
    [switch]$NonInteractive
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "The foundation packaging smoke requires Windows."
}

Import-Module Microsoft.PowerShell.Security -ErrorAction Stop

. (Join-Path $PSScriptRoot "foundation-smoke-lock.ps1")
. (Join-Path $PSScriptRoot "parser-smoke-process.ps1")

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$resolvedRepositoryRoot = (Resolve-Path -LiteralPath $repositoryRoot).Path
$runId = [Guid]::NewGuid().ToString("N")
$evidenceRoot = if ($RetainedEvidenceRoot) {
    if (-not [IO.Path]::IsPathRooted($RetainedEvidenceRoot)) { throw "RetainedEvidenceRoot must be absolute." }
    (Resolve-Path -LiteralPath $RetainedEvidenceRoot -ErrorAction Stop).Path
} else {
    Join-Path $resolvedRepositoryRoot ".tools\evidence\issue-18"
}
$runRoot = Join-Path $evidenceRoot "Package Smoke 音 $runId"
$installDirectory = Join-Path $runRoot "Installed Fruitboard 音"
$rawEvidenceDirectory = Join-Path $runRoot "raw"
$syntheticDataDirectory = Join-Path $env:LOCALAPPDATA "com.fruitboard.desktop.foundation-smoke"
$databasePath = Join-Path $syntheticDataDirectory "storage\fruitboard.db"
$finalEvidencePath = if ([IO.Path]::IsPathRooted($EvidencePath)) { $EvidencePath } else { Join-Path $resolvedRepositoryRoot $EvidencePath }
$cargoTargetDirectory = if ($env:CARGO_TARGET_DIR) {
    if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) }
    else { [IO.Path]::GetFullPath((Join-Path $resolvedRepositoryRoot $env:CARGO_TARGET_DIR)) }
} else { Join-Path $resolvedRepositoryRoot "target" }
$smokeLockPath = Get-FoundationSmokeLockPath
$smokeLockOwner = New-FoundationSmokeLockOwner -RunId $runId -Purpose "windows-foundation-smoke" -RunRoot $runRoot -InstallDirectory $installDirectory
$smokeLockAcquired = $false
$archivedPreviousData = $false

function Assert-ContainedPath {
    param([string]$Candidate, [string]$Parent)

    $candidateFull = [System.IO.Path]::GetFullPath($Candidate)
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    if (-not $candidateFull.StartsWith($parentFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A smoke-test path escaped its expected parent."
    }
}

function Assert-NoReparsePath {
    param([string]$Candidate)
    $entry = Get-Item -LiteralPath $Candidate -Force -ErrorAction Stop
    while ($null -ne $entry) {
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "A smoke file operation encountered an unexpected reparse point." }
        $entry = if ($entry -is [IO.DirectoryInfo]) { $entry.Parent } else { $entry.Directory }
    }
}

function Invoke-CheckedProcess {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) {
        throw "A packaging smoke subprocess failed with exit code $($process.ExitCode)."
    }
}

function New-AppProcess {
    param(
        [string]$FilePath,
        [hashtable]$Environment
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.UseShellExecute = $false
    foreach ($entry in $Environment.GetEnumerator()) {
        $startInfo.Environment[$entry.Key] = [string]$entry.Value
    }
    return [System.Diagnostics.Process]::Start($startInfo)
}

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new(
        [System.Net.IPAddress]::Loopback,
        0
    )
    $listener.Start()
    try {
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    }
    finally {
        $listener.Stop()
    }
}

function Stop-AppGracefully {
    param([System.Diagnostics.Process]$Process)

    if ($Process.HasExited) {
        throw "The packaged shell exited before the close smoke."
    }
    if (-not $Process.CloseMainWindow()) {
        throw "The packaged shell did not expose a closable main window."
    }
    if (-not $Process.WaitForExit(5000)) {
        $Process.Kill()
        $Process.WaitForExit()
        throw "The packaged shell did not close within five seconds."
    }
}

function Invoke-LaunchProbe {
    param(
        [string]$ApplicationPath,
        [string]$NodePath
    )

    $port = Get-FreeTcpPort
    $process = New-AppProcess -FilePath $ApplicationPath -Environment @{
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$port"
    }
    try {
        $probeOutput = & $NodePath (Join-Path $PSScriptRoot "probe-webview-audio.mjs") $port
        if ($LASTEXITCODE -ne 0) {
            if ($process.HasExited) {
                throw "The packaged shell exited before WebView2 became inspectable."
            }
            throw "The WebView2 capability probe failed."
        }
        $probe = $probeOutput | ConvertFrom-Json
        Stop-AppGracefully -Process $process
        return [ordered]@{
            webviewReadyMilliseconds = $probe.webviewReadyMs
            appReadyMilliseconds = $probe.appReadyMs
            probe = $probe
        }
    }
    finally {
        if (-not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
        $process.Dispose()
    }
}

function Invoke-AppSmokeMode {
    param(
        [string]$ApplicationPath,
        [ValidateSet("seed", "verify", "parser-missing")]
        [string]$Mode,
        [string]$OutputPath
    )

    # 9.25s inert probe + six 3s parser requests + 8s crash acknowledgement,
    # plus storage, Tauri startup, evidence sync and exit. Hard outer bound.
    # Diagnostics report only the mode, elapsed milliseconds, exit state, and
    # evidence presence/size: never absolute paths or file contents.
    $timeoutSeconds = 60
    if (Test-Path -LiteralPath $OutputPath) {
        throw "The raw evidence destination must not already exist."
    }
    $process = New-AppProcess -FilePath $ApplicationPath -Environment @{
        FRUITBOARD_FOUNDATION_SMOKE_MODE = $Mode
        FRUITBOARD_FOUNDATION_SMOKE_OUTPUT = $OutputPath
    }
    $applicationStartedUtc = $process.StartTime.ToUniversalTime()
    $parserPath = Join-Path (Split-Path -Parent $ApplicationPath) "fruitboard-flp-parser.exe"
    $handshakePath = [IO.Path]::ChangeExtension($OutputPath, "parser-crash.json")
    $acknowledgementPath = [IO.Path]::ChangeExtension($OutputPath, "parser-crashed")
    $crashAcknowledged = $false
    try {
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $evidenceSeenMs = $null
        while (-not $process.WaitForExit(250)) {
            if (-not $crashAcknowledged -and (Test-Path -LiteralPath $handshakePath -PathType Leaf)) {
                # The file may be visible before the writer finishes. Retry
                # incomplete JSON within the same hard launch deadline.
                $handshake = try { Get-Content -LiteralPath $handshakePath -Raw | ConvertFrom-Json } catch { $null }
                if ($null -ne $handshake) {
                    Invoke-OwnedParserCrash -ApplicationPid $process.Id -ApplicationStartedUtc $applicationStartedUtc -ParserPath $parserPath -ChildPid $handshake.childPid -AcknowledgementPath $acknowledgementPath
                    $crashAcknowledged = $true
                }
            }
            if ($null -eq $evidenceSeenMs -and (Test-Path -LiteralPath $OutputPath)) {
                $evidenceSeenMs = $stopwatch.ElapsedMilliseconds
            }
            if ($stopwatch.Elapsed.TotalSeconds -ge $timeoutSeconds) {
                break
            }
        }
        $stopwatch.Stop()
        $elapsedMs = $stopwatch.ElapsedMilliseconds
        $hasExited = $process.HasExited
        $exitCode = if ($hasExited) { $process.ExitCode } else { -1 }
        $evidenceExists = Test-Path -LiteralPath $OutputPath
        $evidenceBytes = if ($evidenceExists) { (Get-Item -LiteralPath $OutputPath).Length } else { -1 }
        $evidenceSeenText = if ($null -eq $evidenceSeenMs) { "not-seen" } else { "$evidenceSeenMs" }
        if (-not $hasExited) {
            throw "The installed sidecar smoke timed out (mode=$Mode, elapsedMs=$elapsedMs, hasExited=False, evidenceExists=$evidenceExists, evidenceBytes=$evidenceBytes, evidenceSeenMs=$evidenceSeenText)."
        }
        if ($exitCode -ne 0 -or -not $evidenceExists) {
            throw "The installed sidecar smoke failed closed (mode=$Mode, elapsedMs=$elapsedMs, exitCode=$exitCode, evidenceExists=$evidenceExists, evidenceBytes=$evidenceBytes, evidenceSeenMs=$evidenceSeenText)."
        }
        $evidence = Get-Content -LiteralPath $OutputPath -Raw | ConvertFrom-Json
        if ($evidence.status -ne "ok") {
            throw "The installed sidecar smoke returned an error (mode=$Mode, elapsedMs=$elapsedMs, exitCode=$exitCode, evidenceBytes=$evidenceBytes)."
        }
        if ($Mode -eq "parser-missing") {
            if (-not $evidence.parser.missingBinaryContained -or $crashAcknowledged) { throw "Missing parser containment was not verified." }
        } else {
            foreach ($flag in @("fixedInstalledSibling", "healthValidated", "descriptorValidated", "childReused", "rejectionContained", "abruptExitRecovered", "explicitShutdown", "restartAfterShutdown")) {
                if ($evidence.parser.$flag -ne $true) { throw "The installed parser lifecycle assertion failed: $flag." }
            }
            if (-not $crashAcknowledged) { throw "The real owned parser crash was not exercised." }
        }
        Assert-ParserExit -ApplicationPid $process.Id -ApplicationStartedUtc $applicationStartedUtc -ParserPath $parserPath
        return $evidence
    }
    finally {
        try {
            Stop-OwnedParserProcesses -ApplicationPid $process.Id -ApplicationStartedUtc $applicationStartedUtc -ParserPath $parserPath
        } finally {
            if (-not $process.HasExited) {
                $process.Kill()
                if (-not $process.WaitForExit(5000)) { throw "The owned application cleanup deadline expired." }
            }
            $process.Dispose()
        }
    }
}

function Install-SmokePackage {
    param([string]$InstallerPath)

    Invoke-CheckedProcess -FilePath $InstallerPath -ArgumentList @(
        "/S",
        "/D=$installDirectory"
    )
    if (-not (Test-Path -LiteralPath $installDirectory -PathType Container)) {
        throw "The NSIS installer did not create the Unicode install directory."
    }
}

function Uninstall-SmokePackage {
    $uninstaller = Join-Path $installDirectory "uninstall.exe"
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        throw "The NSIS uninstaller is missing."
    }
    Invoke-CheckedProcess -FilePath $uninstaller -ArgumentList @("/S")
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((Test-Path -LiteralPath $installDirectory) -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 100
    }
    if (Test-Path -LiteralPath $installDirectory) {
        throw "The NSIS uninstaller did not remove the application directory."
    }
}

Assert-ContainedPath -Candidate $runRoot -Parent $evidenceRoot
Assert-ContainedPath -Candidate $installDirectory -Parent $runRoot
Assert-ContainedPath -Candidate $syntheticDataDirectory -Parent $env:LOCALAPPDATA

New-Item -ItemType Directory -Force -Path $runRoot, $rawEvidenceDirectory | Out-Null

# Exclusive host lock first: a second run fails here before launching,
# installing, archiving data, or uninstalling another run's package.
# Stale ownership is decided through Get-Process on the recorded PID; never
# by deleting storage/owner.lock and never by killing unrelated processes.
$null = Acquire-FoundationSmokeLock -LockPath $smokeLockPath -Owner $smokeLockOwner -ArchiveParent $runRoot -SharedDataDirectory $syntheticDataDirectory
$smokeLockAcquired = $true
try {
if (Test-Path -LiteralPath $installDirectory) {
    throw "The dedicated install directory already exists; preserve it for inspection."
}
if (Test-Path -LiteralPath $syntheticDataDirectory -PathType Container) {
    $liveAfterLock = @(Get-FoundationSmokeLiveAppProcesses)
    if ($liveAfterLock.Count -gt 0) {
        throw "Installed app processes are running; close every fruitboard-desktop process gracefully before archival."
    }
    $legacyArchive = Join-Path $runRoot "archived-legacy-foundation-smoke-$runId"
    $null = Move-PreviousFoundationSmokeData -SourceDirectory $syntheticDataDirectory -DestinationDirectory $legacyArchive -ExpectedParent $env:LOCALAPPDATA -DestinationParent $runRoot
    $archivedPreviousData = $true
}

$buildTimer = [System.Diagnostics.Stopwatch]::StartNew()
Push-Location $resolvedRepositoryRoot
try {
    & pnpm.cmd package:windows:smoke
    if ($LASTEXITCODE -ne 0) {
        throw "The locked NSIS package build failed."
    }
}
finally {
    Pop-Location
}
$buildTimer.Stop()

$builtInstaller = Get-ChildItem -LiteralPath (Join-Path $cargoTargetDirectory "release\bundle\nsis") -Filter "*.exe" |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
if ($null -eq $builtInstaller) {
    throw "The NSIS package build did not produce an installer."
}
$installerPath = Join-Path $runRoot "Fruitboard Foundation Smoke setup 音.exe"
Copy-Item -LiteralPath $builtInstaller.FullName -Destination $installerPath

$installerSignature = Get-AuthenticodeSignature -LiteralPath $installerPath
if ($installerSignature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
    throw "The development smoke expected an explicitly unsigned installer."
}

Install-SmokePackage -InstallerPath $installerPath
$applicationPath = Join-Path $installDirectory "fruitboard-desktop.exe"
if (-not (Test-Path -LiteralPath $applicationPath -PathType Leaf)) {
    throw "The installed Fruitboard executable is missing."
}
$sidecarPath = Join-Path $installDirectory "fruitboard-sidecar-smoke.exe"
if (-not (Test-Path -LiteralPath $sidecarPath -PathType Leaf)) {
    throw "The installed inert sidecar is missing."
}
$applicationBytes = (Get-Item -LiteralPath $applicationPath).Length
$sidecarBytes = (Get-Item -LiteralPath $sidecarPath).Length
$parserPath = Join-Path $installDirectory "fruitboard-flp-parser.exe"
$parserItem = Get-Item -LiteralPath $parserPath -ErrorAction Stop
if ($parserItem.PSIsContainer -or ($parserItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "The installed parser is not an ordinary file." }
$parserBytes = $parserItem.Length
$parserHash = (Get-FileHash -LiteralPath $parserPath -Algorithm SHA256).Hash
$preparedParserPath = Join-Path $resolvedRepositoryRoot "apps\desktop\src-tauri\binaries\fruitboard-flp-parser-x86_64-pc-windows-msvc.exe"
if ($parserHash -ne (Get-FileHash -LiteralPath $preparedParserPath -Algorithm SHA256).Hash) { throw "The installed parser differs from the locked build." }
$installedSignature = Get-AuthenticodeSignature -LiteralPath $applicationPath
if ($installedSignature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
    throw "The development smoke expected an explicitly unsigned application."
}

$coldLaunch = $null
$warmLaunch = $null
if (-not $NonInteractive) {
    $nodePath = (Get-Command node.exe -ErrorAction Stop).Source
    $coldLaunch = Invoke-LaunchProbe -ApplicationPath $applicationPath -NodePath $nodePath
    $warmLaunch = Invoke-LaunchProbe -ApplicationPath $applicationPath -NodePath $nodePath
}
$seedEvidencePath = Join-Path $rawEvidenceDirectory "seed.json"
$seedEvidence = Invoke-AppSmokeMode -ApplicationPath $applicationPath -Mode "seed" -OutputPath $seedEvidencePath

# Hold only this verified synthetic installation's parser. Restore it even
# when the missing-file test fails; no source or application data is removed.
$heldParserPath = Join-Path $installDirectory "fruitboard-flp-parser.held"
Assert-ContainedPath -Candidate $parserPath -Parent $installDirectory
Assert-ContainedPath -Candidate $heldParserPath -Parent $installDirectory
Assert-NoReparsePath -Candidate $parserPath
if (Test-Path -LiteralPath $heldParserPath) { throw "The held parser destination already exists." }
Move-Item -LiteralPath $parserPath -Destination $heldParserPath
try {
    $missingEvidence = Invoke-AppSmokeMode -ApplicationPath $applicationPath -Mode "parser-missing" -OutputPath (Join-Path $rawEvidenceDirectory "parser-missing.json")
} finally {
    Assert-NoReparsePath -Candidate $heldParserPath
    if (Test-Path -LiteralPath $parserPath) { throw "Parser restoration refused to replace an unexpected file." }
    Move-Item -LiteralPath $heldParserPath -Destination $parserPath
}
if ((Get-FileHash -LiteralPath $parserPath -Algorithm SHA256).Hash -ne $parserHash) { throw "Parser restoration changed its bytes." }

if (-not (Test-Path -LiteralPath $databasePath -PathType Leaf)) {
    throw "The packaged app did not create its isolated SQLite database."
}
$databaseHashBeforeUninstall = (Get-FileHash -LiteralPath $databasePath -Algorithm SHA256).Hash
$databaseSize = (Get-Item -LiteralPath $databasePath).Length
$installedBytes = (Get-ChildItem -LiteralPath $installDirectory -File -Recurse |
    Measure-Object -Property Length -Sum).Sum

Uninstall-SmokePackage
if (-not (Test-Path -LiteralPath $databasePath -PathType Leaf)) {
    throw "Uninstall removed the synthetic user database."
}
if ((Get-FileHash -LiteralPath $databasePath -Algorithm SHA256).Hash -ne $databaseHashBeforeUninstall) {
    throw "Uninstall changed the synthetic user database."
}

Install-SmokePackage -InstallerPath $installerPath
$verifyEvidencePath = Join-Path $rawEvidenceDirectory "verify.json"
$verifyEvidence = Invoke-AppSmokeMode -ApplicationPath $applicationPath -Mode "verify" -OutputPath $verifyEvidencePath
if ($verifyEvidence.storage.startupViewBefore -ne "library" -or $verifyEvidence.storage.startupViewAfter -ne "library") {
    throw "The reinstalled app did not recover the persisted startup preference."
}
if ((Get-FileHash -LiteralPath $databasePath -Algorithm SHA256).Hash -ne $databaseHashBeforeUninstall) {
    throw "Reinstall or relaunch changed the persisted database unexpectedly."
}
Uninstall-SmokePackage
if ((Get-FileHash -LiteralPath $databasePath -Algorithm SHA256).Hash -ne $databaseHashBeforeUninstall) {
    throw "The second uninstall changed the synthetic user database."
}

$defender = try {
    $status = Get-MpComputerStatus -ErrorAction Stop
    [ordered]@{
        provider = "Microsoft Defender"
        antivirusEnabled = [bool]$status.AntivirusEnabled
        realTimeProtectionEnabled = [bool]$status.RealTimeProtectionEnabled
        installerOrLaunchBlockObserved = $false
    }
}
catch {
    [ordered]@{
        provider = "not observable"
        installerOrLaunchBlockObserved = $false
    }
}

$launchEvidence = if ($NonInteractive) {
    [ordered]@{
        mode = "hosted-service-session"
        interactiveWindowObserved = $false
        nativeSeedAndVerifyLaunches = $true
    }
}
else {
    [ordered]@{
        mode = "interactive"
        interactiveWindowObserved = $true
        coldWebviewReadyMilliseconds = $coldLaunch.webviewReadyMilliseconds
        coldAppReadyMilliseconds = $coldLaunch.appReadyMilliseconds
        warmWebviewReadyMilliseconds = $warmLaunch.webviewReadyMilliseconds
        warmAppReadyMilliseconds = $warmLaunch.appReadyMilliseconds
        coldAppUrl = $coldLaunch.probe.url
        warmAppUrl = $warmLaunch.probe.url
        shellRendered = $true
        gracefulClose = $true
    }
}

$audioEvidence = if ($NonInteractive) {
    [ordered]@{
        status = "not-probed"
        reason = "hosted-service-session"
    }
}
else {
    $coldLaunch.probe.audioCanPlayType
}

$webViewUserAgent = if ($NonInteractive) {
    "not-probed-hosted-service-session"
}
else {
    $coldLaunch.probe.userAgent
}

$evidence = [ordered]@{
    schemaVersion = 2
    status = "ok"
    platform = [ordered]@{
        os = "Windows"
        architecture = $env:PROCESSOR_ARCHITECTURE
        webViewUserAgent = $webViewUserAgent
    }
    package = [ordered]@{
        format = "NSIS current-user"
        unsigned = $true
        installerBytes = $builtInstaller.Length
        installedBytes = [long]$installedBytes
        applicationBytes = $applicationBytes
        sidecarBytes = $sidecarBytes
        parserBytes = $parserBytes
        parserSha256 = $parserHash
        buildMilliseconds = $buildTimer.ElapsedMilliseconds
        spacesAndUnicodeInstallerPath = $true
        spacesAndUnicodeInstallPath = $true
        webViewInstallMode = "downloadBootstrapper"
    }
    launch = $launchEvidence
    audioCanPlayType = $audioEvidence
    sidecar = $seedEvidence.sidecar
    parser = [ordered]@{
        seed = $seedEvidence.parser
        missing = $missingEvidence.parser
        reinstall = $verifyEvidence.parser
        restoredBytesVerified = $true
        noOwnedChildAfterExit = $true
    }
    dataSafety = [ordered]@{
        databaseBytes = $databaseSize
        firstUninstallPreservedDatabase = $true
        reinstallRestoredStartupView = $true
        secondUninstallPreservedDatabase = $true
        syntheticStateRetainedForReview = $true
    }
    isolation = [ordered]@{
        mode = "exclusive-host-lock"
        lockFile = "com.fruitboard.desktop.foundation-smoke.lock.json"
        runId = $runId
        purpose = "windows-foundation-smoke"
        archivedPreviousData = $archivedPreviousData
        cleanupOwner = "This run releases only its own lock; prior archives remain for owner review; no database or evidence deletion."
        restorationOwner = "To restore a prior archive: close every fruitboard-desktop process, move the live smoke directory to a new run-specific archive, then move the chosen archive back; never delete owner.lock."
    }
    antivirus = $defender
    limitations = @(
        "Unsigned development evidence only; Windows trust warnings remain expected.",
        "Audio results are WebView2 canPlayType capability signals, not decoded playback tests.",
        "Launch readiness separates WebView target appearance from rendered-shell observation; blank, loading, or error pages fail the probe closed.",
        "The download-bootstrapper installer requires network access when WebView2 is absent.",
        "The real parser is packaged for native lifecycle validation only; no scanner analysis job or player is enabled."
        "Concurrent installed validation runs share one test identity and are serialized by the exclusive host lock; a second run fails before side effects."
    )
}

$finalEvidenceDirectory = Split-Path -Parent $finalEvidencePath
New-Item -ItemType Directory -Force -Path $finalEvidenceDirectory | Out-Null
$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $finalEvidencePath -Encoding utf8
$evidence | ConvertTo-Json -Depth 8
}
finally {
    if ($smokeLockAcquired) {
        Release-FoundationSmokeLock -LockPath $smokeLockPath -RunId $runId
    }
}
