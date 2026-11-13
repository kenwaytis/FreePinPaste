param(
    [string]$Script = "$PSScriptRoot\windows-smoke.py",
    [string]$Arguments = '--inventory',
    [int]$TimeoutSeconds = 180,
    [string]$Python = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$name = 'FreePinPaste-Validation-' + [guid]::NewGuid().ToString('N')
if (!$Python) {
    $Python = Join-Path (Split-Path (Get-Command python.exe -ErrorAction Stop).Source) 'pythonw.exe'
}
if (!(Test-Path $Python)) { throw "Python executable not found: $Python" }
$action = New-ScheduledTaskAction -Execute $Python -Argument "-u `"$Script`" $Arguments" -WorkingDirectory $root
$principal = New-ScheduledTaskPrincipal -UserId ([System.Security.Principal.WindowsIdentity]::GetCurrent().Name) -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Seconds ($TimeoutSeconds + 30)) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
try {
    Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Settings $settings | Out-Null
    $started = Get-Date
    Start-ScheduledTask -TaskName $name
    do {
        Start-Sleep -Milliseconds 500
        $task = Get-ScheduledTask -TaskName $name
        $info = Get-ScheduledTaskInfo -TaskName $name
        if (((Get-Date) - $started).TotalSeconds -gt $TimeoutSeconds) { throw "Desktop task timed out: $name" }
    } while ($task.State -eq 'Running' -or $info.LastRunTime.Year -lt 2000)
    if ($info.LastTaskResult -ne 0) { throw "Desktop test failed: $($info.LastTaskResult); see .cache/windows-validation/" }
} finally {
    if (Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue) {
        Stop-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $name -Confirm:$false
    }
}
