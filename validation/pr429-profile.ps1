$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:GITHUB_REF -ne 'refs/heads/validation/pr429-installed') {
  throw 'This profile override is restricted to the disposable validation runner.'
}
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ValidationLocalAppData
{
    private static Guid Folder = new Guid("F1B32785-6FBA-4FCF-9D55-7B8E7F157091");
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHSetKnownFolderPath(ref Guid folder, uint flags, IntPtr user, string path);
    [DllImport("shell32.dll")]
    private static extern int SHGetKnownFolderPath(ref Guid folder, uint flags, IntPtr user, out IntPtr path);
    public static void Set(string path)
    {
        Marshal.ThrowExceptionForHR(SHSetKnownFolderPath(ref Folder, 0, IntPtr.Zero, path));
    }
    public static string Get()
    {
        IntPtr path;
        Marshal.ThrowExceptionForHR(SHGetKnownFolderPath(ref Folder, 0, IntPtr.Zero, out path));
        try { return Marshal.PtrToStringUni(path); }
        finally { Marshal.FreeCoTaskMem(path); }
    }
}
"@
$originalFolder = [ValidationLocalAppData]::Get()
$originalEnvironment = $env:LOCALAPPDATA
$isolatedFolder = Join-Path $env:RUNNER_TEMP 'fruitboard-429-localappdata'
New-Item -ItemType Directory -Path $isolatedFolder | Out-Null
try {
  [ValidationLocalAppData]::Set($isolatedFolder)
  if ([ValidationLocalAppData]::Get() -ne $isolatedFolder) { throw 'Windows did not select the isolated profile directory' }
  $env:LOCALAPPDATA = $isolatedFolder
  & (Join-Path $PSScriptRoot 'pr429-host.ps1')
} finally {
  $env:LOCALAPPDATA = $originalEnvironment
  [ValidationLocalAppData]::Set($originalFolder)
  if ([ValidationLocalAppData]::Get() -ne $originalFolder) { throw 'Windows did not restore the original profile directory' }
}
