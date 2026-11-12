param (
    [string]$ExePath = "D:\testik\EqualizerFD-Craft.exe",
    [switch]$Remove = $false
)

$TaskName = "EqualizerFDCraft_AutoStart"

if ($Remove) {
    Write-Host "Removing task $TaskName..."
    Start-Process schtasks.exe -ArgumentList "/Delete /TN `"$TaskName`" /F" -Verb RunAs -Wait -WindowStyle Hidden
    reg.exe delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "EqualizerFDCraft" /f
    Write-Host "Autostart removed."
    exit 0
}

Write-Host "Configuring high-priority instant autostart for: $ExePath"

# 1. Disable Windows Explorer Startup Delay (0 ms instead of default 10-30s delay)
reg.exe add "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize" /v "StartupDelayInMSec" /t REG_DWORD /d 0 /f

# 2. Add standard Run key as secondary backup
reg.exe add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "EqualizerFDCraft" /t REG_SZ /d "`"$ExePath`"" /f

# 3. Create high-priority Scheduled Task XML (Priority 4, RunLevel Highest, Delay 0s)
$xml = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Equalizer FD CRAFT High-Priority Real-Time Audio Engine AutoStart</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <Delay>PT0S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <DisallowStartOnRemoteAppSession>false</DisallowStartOnRemoteAppSession>
    <UseUnifiedSchedulingEngine>true</UseUnifiedSchedulingEngine>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>$ExePath</Command>
    </Exec>
  </Actions>
</Task>
"@

$xmlPath = Join-Path $env:TEMP "EqualizerFDCraft_Task.xml"
[System.IO.File]::WriteAllText($xmlPath, $xml, [System.Text.Encoding]::Unicode)

try {
    $proc = Start-Process schtasks.exe -ArgumentList "/Create /TN `"$TaskName`" /XML `"$xmlPath`" /F" -Verb RunAs -PassThru -Wait -WindowStyle Hidden
    Write-Host "Task Scheduler registration exit code: $($proc.ExitCode)"
} catch {
    Write-Warning "Could not register scheduled task: $($_.Exception.Message)"
}

# Verify task
schtasks.exe /Query /TN "$TaskName"

