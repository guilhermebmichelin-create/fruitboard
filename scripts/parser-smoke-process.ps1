# Only processes belonging to this launched app in this isolated install can
# be terminated. PID, parent, full executable path and creation time all agree.
function Get-OwnedParserProcesses {
    param([int]$ApplicationPid, [DateTime]$ApplicationStartedUtc, [string]$ParserPath)
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$ApplicationPid" -ErrorAction Stop)
    foreach ($child in $children) {
        if ($null -eq $child.ExecutablePath -or
            -not [string]::Equals($child.ExecutablePath, $ParserPath, [StringComparison]::OrdinalIgnoreCase)) {
            continue
        }
        $owned = [Diagnostics.Process]::GetProcessById([int]$child.ProcessId)
        try {
            if ($owned.StartTime.ToUniversalTime() -lt $ApplicationStartedUtc -or
                -not [string]::Equals($owned.MainModule.FileName, $ParserPath, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Parser process ownership could not be verified."
            }
            # Materialize the process handle before returning: later operations
            # keep this identity even if the numeric PID is recycled.
            $null = $owned.Handle
            $owned
            $owned = $null
        }
        finally {
            if ($null -ne $owned) { $owned.Dispose() }
        }
    }
}

function Invoke-OwnedParserCrash {
    param([int]$ApplicationPid, [DateTime]$ApplicationStartedUtc, [string]$ParserPath, [int]$ChildPid, [string]$AcknowledgementPath)
    $owned = @(Get-OwnedParserProcesses -ApplicationPid $ApplicationPid -ApplicationStartedUtc $ApplicationStartedUtc -ParserPath $ParserPath)
    try {
        if ($owned.Count -ne 1 -or $ChildPid -le 0 -or $owned[0].Id -ne $ChildPid) {
            throw "The parser crash handshake did not identify the owned child."
        }
        $owned[0].Kill()
        if (-not $owned[0].WaitForExit(3000)) { throw "The owned parser did not exit within its crash budget." }
        $ack = [IO.File]::Open($AcknowledgementPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        $ack.Dispose()
    }
    finally {
        foreach ($child in $owned) { $child.Dispose() }
    }
}

function Assert-ParserExit {
    param([int]$ApplicationPid, [DateTime]$ApplicationStartedUtc, [string]$ParserPath)
    $remaining = @(Get-OwnedParserProcesses -ApplicationPid $ApplicationPid -ApplicationStartedUtc $ApplicationStartedUtc -ParserPath $ParserPath)
    try {
        if ($remaining.Count -gt 0) { throw "The installed application left an owned parser running." }
    }
    finally {
        foreach ($child in $remaining) { $child.Dispose() }
    }
}

function Stop-OwnedParserProcesses {
    param([int]$ApplicationPid, [DateTime]$ApplicationStartedUtc, [string]$ParserPath)
    $remaining = @(Get-OwnedParserProcesses -ApplicationPid $ApplicationPid -ApplicationStartedUtc $ApplicationStartedUtc -ParserPath $ParserPath)
    try {
        foreach ($child in $remaining) {
            $child.Kill()
            if (-not $child.WaitForExit(3000)) { throw "The owned parser cleanup deadline expired." }
        }
    }
    finally {
        foreach ($child in $remaining) { $child.Dispose() }
    }
}
