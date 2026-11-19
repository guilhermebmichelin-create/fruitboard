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
node scripts/preflight-build-storage.mjs --source $source --build $env:CARGO_TARGET_DIR --cache $env:CARGO_TARGET_DIR --evidence $evidence --estimated-output-gib 0.5
if ($LASTEXITCODE -ne 0) { throw 'Installed storage preflight failed' }
if (Get-Process fruitboard-desktop,fruitboard-flp-parser,cargo,rustc -ErrorAction SilentlyContinue) { throw 'Host is busy' }
$null = Acquire-FoundationSmokeLock -LockPath $lock -Owner $owner -ArchiveParent $evidence -SharedDataDirectory $profile
$installed = $false
try {
  if ((Test-Path -LiteralPath $profile) -or (Test-Path -LiteralPath $install)) { throw 'Expected fresh install and profile' }
  $built = @(Get-ChildItem -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release/bundle/nsis') -Filter '*.exe' -File)
  if ($built.Count -ne 1) { throw 'Expected one newly built NSIS installer' }
  $installer = Join-Path $evidence 'setup.exe'
  Copy-Item -LiteralPath $built[0].FullName -Destination $installer
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
  if ((Get-Hash $app) -ne (Get-Hash (Join-Path $env:CARGO_TARGET_DIR 'release/fruitboard-desktop.exe'))) { throw 'Installed application differs' }
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
    $dbBefore = Get-Hash $db
    Invoke-Installer (Join-Path $install 'uninstall.exe') @('/S')
    if ((Test-Path -LiteralPath $install) -or (Get-Hash $db) -ne $dbBefore) { throw 'Uninstall or database preservation failed' }
    Write-Output 'Uninstalled the review package and verified the database was preserved.'
  }
  Release-FoundationSmokeLock -LockPath $lock -RunId $runId
}
Write-Output ('Free GiB after validation: ' + [math]::Round((Get-PSDrive ([IO.Path]::GetPathRoot($evidence).Substring(0,1))).Free / 1GB, 2))
'PR 429 installed automation counts passed on the exact pinned source.' >> $env:GITHUB_STEP_SUMMARY
