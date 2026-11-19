$ErrorActionPreference = 'Stop'
$evidence = $env:FRUITBOARD_429_EVIDENCE
$source = $PWD.Path
$runId = "pr429-$env:GITHUB_RUN_ID-$env:GITHUB_RUN_ATTEMPT"
$install = Join-Path $evidence 'Installed Fruitboard'
$fixtures = Join-Path $evidence 'Projects'
$profile = Join-Path $env:LOCALAPPDATA 'com.fruitboard.desktop.foundation-smoke'
$names = @('FIX-FL2026-MIN.flp', 'FIX-FL2026-AUTO-2POINT.flp', 'FIX-FL2024-A.flp')
. (Join-Path $source 'scripts/foundation-smoke-lock.ps1')
$lock = Get-FoundationSmokeLockPath
$owner = New-FoundationSmokeLockOwner -RunId $runId -Purpose 'PR 429 installed automation counts' -RunRoot $evidence -InstallDirectory $install
function Invoke-Installer([string]$Path, [string[]]$Arguments) {
  $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -PassThru
  $null = $process.Handle
  if (-not $process.WaitForExit(180000) -or $process.ExitCode -ne 0) { throw 'Installer process failed' }
}
function Get-Hash([string]$Path) { (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash }
function Invoke-Review([string]$Mode) {
  $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
  $listener.Start()
  $port = $listener.LocalEndpoint.Port
  $listener.Stop()
  $info = [Diagnostics.ProcessStartInfo]::new($app)
  $info.WorkingDirectory = $install
  $info.UseShellExecute = $false
  $info.EnvironmentVariables['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'] = "--remote-debugging-port=$port --remote-debugging-address=127.0.0.1"
  $info.EnvironmentVariables['FRUITBOARD_FOUNDATION_SMOKE_MODE'] = 'shutdown-diagnostics'
  $info.EnvironmentVariables['FRUITBOARD_FOUNDATION_SMOKE_OUTPUT'] = Join-Path $evidence "$Mode-smoke.json"
  $appProcess = [Diagnostics.Process]::Start($info)
  try {
    node (Join-Path $PSScriptRoot 'pr429-library.mjs') $port $Mode $fixtures $evidence
    if ($LASTEXITCODE -ne 0) { throw "Library validation failed for $Mode" }
  } finally {
    if ($appProcess.HasExited -or -not $appProcess.CloseMainWindow() -or -not $appProcess.WaitForExit(10000) -or $appProcess.ExitCode -ne 0) {
      throw "Graceful shutdown failed for $Mode"
    }
    node scripts/validate-shutdown-trace.mjs (Join-Path $evidence "$Mode-smoke.shutdown-stages.jsonl") --analysis-jobs
    if ($LASTEXITCODE -ne 0) { throw "Shutdown trace validation failed for $Mode" }
  }
}
# The build window is closed. Installation uses an independent copy of the
# finished package and does not write to the Cargo cache.
$reviewVolumes = @($source, $evidence, $env:LOCALAPPDATA, $env:TEMP) | ForEach-Object {
  [IO.Path]::GetPathRoot($_).Substring(0,1)
} | Sort-Object -Unique
foreach ($volume in $reviewVolumes) {
  $free = (Get-PSDrive $volume).Free / 1GB
  Write-Output ("Installed preflight: {0}: {1:N2} GiB free" -f $volume, $free)
  if ($free -lt 30.5) { throw "Insufficient installed-test reserve on ${volume}:" }
}
if (Get-Process fruitboard-desktop,fruitboard-flp-parser,cargo,rustc -ErrorAction SilentlyContinue) { throw 'Host is busy' }
$null = Acquire-FoundationSmokeLock -LockPath $lock -Owner $owner -ArchiveParent $evidence -SharedDataDirectory $profile
$installed = $false
try {
  if ((Test-Path -LiteralPath $profile) -or (Test-Path -LiteralPath $install)) { throw 'Expected fresh install and profile' }
  $built = @(Get-ChildItem -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release/bundle/nsis') -Filter '*.exe' -File)
  if ($built.Count -ne 1) { throw 'Expected one newly built NSIS installer' }
  $installer = Join-Path $evidence 'setup.exe'
  Copy-Item -LiteralPath $built[0].FullName -Destination $installer
  if ((Get-Hash $built[0].FullName) -ne (Get-Hash $installer)) { throw 'Retained installer copy differs' }
  New-Item -ItemType Directory -Path $fixtures | Out-Null
  $hashes = @{}
  foreach ($name in $names) {
    $original = Join-Path $source "fixtures/parser-corpus/$name"
    $hashes[$name] = Get-Hash $original
    $copy = Join-Path $fixtures $name
    Copy-Item -LiteralPath $original -Destination $copy
    if ((Get-Hash $copy) -ne $hashes[$name]) { throw 'Fixture copy differs' }
  }
  Invoke-Installer $installer @('/S', "/D=$install")
  $installed = $true
  $app = Join-Path $install 'fruitboard-desktop.exe'
  $parser = Join-Path $install 'fruitboard-flp-parser.exe'
  if ((Get-AuthenticodeSignature -LiteralPath $app).Status -ne 'NotSigned') { throw 'Expected unsigned development app' }
  if ((Get-Hash $parser) -ne (Get-Hash (Join-Path $source 'apps/desktop/src-tauri/binaries/fruitboard-flp-parser-x86_64-pc-windows-msvc.exe'))) { throw 'Installed parser differs' }
  Write-Output ("Source: " + (git rev-parse HEAD))
  Write-Output ("Installer SHA-256: " + (Get-Hash $installer))
  Write-Output ("Installed application SHA-256: " + (Get-Hash $app))
  Write-Output ("Installed parser SHA-256: " + (Get-Hash $parser))
  Invoke-Review 'first'
  Invoke-Review 'restart'
  foreach ($name in $names) {
    if ((Get-Hash (Join-Path $source "fixtures/parser-corpus/$name")) -ne $hashes[$name] -or (Get-Hash (Join-Path $fixtures $name)) -ne $hashes[$name]) { throw 'Fixture bytes changed' }
    Write-Output "$name SHA-256 $($hashes[$name])"
  }
} finally {
  if ($installed -and -not (Get-Process fruitboard-desktop,fruitboard-flp-parser -ErrorAction SilentlyContinue)) {
    $db = Join-Path $profile 'storage/fruitboard.db'
    $dbBefore = if (Test-Path -LiteralPath $db -PathType Leaf) { Get-Hash $db } else { $null }
    Invoke-Installer (Join-Path $install 'uninstall.exe') @('/S')
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ((Test-Path -LiteralPath $install) -and [DateTime]::UtcNow -lt $deadline) {
      Start-Sleep -Milliseconds 200
    }
    if (Test-Path -LiteralPath $install) { throw 'Uninstall failed' }
    if ($null -ne $dbBefore) {
      if ((Get-Hash $db) -ne $dbBefore) { throw 'Uninstall changed the database' }
      Write-Output 'Uninstalled the review package and verified the database was preserved.'
    }
  }
  Release-FoundationSmokeLock -LockPath $lock -RunId $runId
}
Write-Output ('Free GiB after validation: ' + [math]::Round((Get-PSDrive ([IO.Path]::GetPathRoot($evidence).Substring(0,1))).Free / 1GB, 2))
'PR 429 installed automation counts passed on the exact pinned source.' >> $env:GITHUB_STEP_SUMMARY
