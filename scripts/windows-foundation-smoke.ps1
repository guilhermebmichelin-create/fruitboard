[CmdletBinding()]
param(
    [string]$EvidencePath = "docs/review/issue-18/windows-smoke.json",
    [switch]$NonInteractive
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "The foundation packaging smoke requires Windows."
}

Import-Module Microsoft.PowerShell.Security -ErrorAction Stop

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$resolvedRepositoryRoot = (Resolve-Path -LiteralPath $repositoryRoot).Path
$runId = [Guid]::NewGuid().ToString("N")
$runRoot = Join-Path $resolvedRepositoryRoot ".tools\evidence\issue-18\Package Smoke 音 $runId"
$installDirectory = Join-Path $runRoot "Installed Fruitboard 音"
$rawEvidenceDirectory = Join-Path $runRoot "raw"
$syntheticDataDirectory = Join-Path $env:LOCALAPPDATA "com.fruitboard.desktop.foundation-smoke"
$databasePath = Join-Path $syntheticDataDirectory "storage\fruitboard.db"
$finalEvidencePath = Join-Path $resolvedRepositoryRoot $EvidencePath

function Assert-ContainedPath {
    param([string]$Candidate, [string]$Parent)

    $candidateFull = [System.IO.Path]::GetFullPath($Candidate)
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    if (-not $candidateFull.StartsWith($parentFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "A smoke-test path escaped its expected parent."
    }
}

function Invoke-CheckedProcess {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList
    )

    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -PassThru -Wait
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
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
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
        $stopwatch.Stop()
        $probe = $probeOutput | ConvertFrom-Json
        Stop-AppGracefully -Process $process
        return [ordered]@{
            readyMilliseconds = $stopwatch.ElapsedMilliseconds
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
        [ValidateSet("seed", "verify")]
        [string]$Mode,
        [string]$OutputPath
    )

    if (Test-Path -LiteralPath $OutputPath) {
        throw "The raw evidence destination must not already exist."
    }
    $process = New-AppProcess -FilePath $ApplicationPath -Environment @{
        FRUITBOARD_FOUNDATION_SMOKE_MODE = $Mode
        FRUITBOARD_FOUNDATION_SMOKE_OUTPUT = $OutputPath
    }
    try {
        if (-not $process.WaitForExit(10000)) {
            $process.Kill()
            $process.WaitForExit()
            throw "The installed sidecar smoke timed out."
        }
        if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $OutputPath)) {
            throw "The installed sidecar smoke failed closed."
        }
        $evidence = Get-Content -LiteralPath $OutputPath -Raw | ConvertFrom-Json
        if ($evidence.status -ne "ok") {
            throw "The installed sidecar smoke returned an error."
        }
        return $evidence
    }
    finally {
        $process.Dispose()
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

Assert-ContainedPath -Candidate $runRoot -Parent (Join-Path $resolvedRepositoryRoot ".tools")
Assert-ContainedPath -Candidate $installDirectory -Parent $runRoot
Assert-ContainedPath -Candidate $syntheticDataDirectory -Parent $env:LOCALAPPDATA

if (Test-Path -LiteralPath $installDirectory) {
    throw "The dedicated install directory already exists; preserve it for inspection."
}
if (Test-Path -LiteralPath $syntheticDataDirectory) {
    throw "The dedicated synthetic data directory already exists; preserve it for inspection."
}

New-Item -ItemType Directory -Force -Path $runRoot, $rawEvidenceDirectory | Out-Null
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

$builtInstaller = Get-ChildItem -LiteralPath (Join-Path $resolvedRepositoryRoot "target\release\bundle\nsis") -Filter "*.exe" |
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
        coldReadyMilliseconds = $coldLaunch.readyMilliseconds
        warmReadyMilliseconds = $warmLaunch.readyMilliseconds
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
    schemaVersion = 1
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
        buildMilliseconds = $buildTimer.ElapsedMilliseconds
        spacesAndUnicodeInstallerPath = $true
        spacesAndUnicodeInstallPath = $true
        webViewInstallMode = "downloadBootstrapper"
    }
    launch = $launchEvidence
    audioCanPlayType = $audioEvidence
    sidecar = $seedEvidence.sidecar
    dataSafety = [ordered]@{
        databaseBytes = $databaseSize
        firstUninstallPreservedDatabase = $true
        reinstallRestoredStartupView = $true
        secondUninstallPreservedDatabase = $true
        syntheticStateRetainedForReview = $true
    }
    antivirus = $defender
    limitations = @(
        "Unsigned development evidence only; Windows trust warnings remain expected.",
        "Audio results are WebView2 canPlayType capability signals, not decoded playback tests.",
        "The download-bootstrapper installer requires network access when WebView2 is absent.",
        "No updater, signing credential, Python runtime, PyFLP, scanner, parser, or player is included."
    )
}

$finalEvidenceDirectory = Split-Path -Parent $finalEvidencePath
New-Item -ItemType Directory -Force -Path $finalEvidenceDirectory | Out-Null
$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $finalEvidencePath -Encoding utf8
$evidence | ConvertTo-Json -Depth 8
