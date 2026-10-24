# Windows System Tray Notification Service for Equalizer FD CRAFT
# Provides persistent taskbar notification icon, dynamic status tooltip, and native right-click control menu

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$port = 49200
$appUrl = "http://127.0.0.1:$port"
$appExe = "D:\testik\EqualizerFD-Craft.exe"
$icoPath = "D:\testik\EqualizerFD-Craft.ico"

# Create NotifyIcon
$notify = New-Object System.Windows.Forms.NotifyIcon
if (Test-Path $icoPath) {
    $notify.Icon = [System.Drawing.Icon]::new($icoPath)
} else {
    $notify.Icon = [System.Drawing.SystemIcons]::Application
}
$notify.Text = "Equalizer FD CRAFT - Studio Audio Engine"

# Helper: check autostart
function Get-AutostartStatus {
    try {
        $val = Get-ItemPropertyValue -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EqualizerFDCraft" -ErrorAction SilentlyContinue
        return ($null -ne $val)
    } catch {
        return $false
    }
}

function Set-AutostartStatus($enable) {
    try {
        if ($enable) {
            Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EqualizerFDCraft" -Value "`"$appExe`"" -Type String -Force | Out-Null
        } else {
            Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EqualizerFDCraft" -ErrorAction SilentlyContinue | Out-Null
        }
    } catch {}
}

# P/Invoke Win32 for focusing application window
Add-Type -MemberDefinition @'
public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

[DllImport("user32.dll")]
public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

[DllImport("user32.dll", CharSet = CharSet.Unicode)]
public static extern int GetWindowTextW(IntPtr hWnd, System.Text.StringBuilder lpString, int nMaxCount);

[DllImport("user32.dll")]
public static extern bool IsWindowVisible(IntPtr hWnd);

[DllImport("user32.dll")]
public static extern bool SetForegroundWindow(IntPtr hWnd);

[DllImport("user32.dll")]
public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

public static bool FocusExistingApp(string matchTitle) {
    IntPtr targetHwnd = IntPtr.Zero;
    EnumWindows((hWnd, lParam) => {
        if (IsWindowVisible(hWnd)) {
            var sb = new System.Text.StringBuilder(512);
            GetWindowTextW(hWnd, sb, 512);
            string title = sb.ToString();
            if (!string.IsNullOrEmpty(title) && title.IndexOf(matchTitle, StringComparison.OrdinalIgnoreCase) >= 0) {
                targetHwnd = hWnd;
                return false;
            }
        }
        return true;
    }, IntPtr.Zero);

    if (targetHwnd != IntPtr.Zero) {
        ShowWindow(targetHwnd, 9); // SW_RESTORE
        SetForegroundWindow(targetHwnd);
        return true;
    }
    return false;
}
'@ -Name "Win32Focus" -Namespace "EqualizerFDCraft" -ErrorAction SilentlyContinue

function Open-AppWindow($subPath = "") {
    $targetUrl = "$appUrl$subPath"

    # 1. Bring existing window to focus if already running
    try {
        if ([EqualizerFDCraft.Win32Focus]::FocusExistingApp("Equalizer FD CRAFT")) {
            return
        }
    } catch {}

    # 2. Launch standalone PC application window (isolated profile, no browser tabs/URL bar)
    $profileDir = Join-Path $env:LOCALAPPDATA "EqualizerFDCraft\profile"
    if (-not (Test-Path $profileDir)) {
        New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
    }

    $browsers = @(
        "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        "C:\Program Files\Google\Chrome\Application\chrome.exe",
        "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"
    )

    $launched = $false
    foreach ($b in $browsers) {
        if (Test-Path $b) {
            $args = @(
                "--app=$targetUrl",
                "--user-data-dir=$profileDir",
                "--window-size=1280,820",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-features=Translate,OptimizationHints,MediaRouter"
            )
            try {
                Start-Process -FilePath $b -ArgumentList $args
                $launched = $true
                break
            } catch {}
        }
    }

    if (-not $launched) {
        Start-Process $targetUrl
    }
}

# Context Menu
$menu = New-Object System.Windows.Forms.ContextMenuStrip

# 1. Open Studio Editor (Primary / Bold)
$itemOpen = $menu.Items.Add("Открыть Configuration Editor")
$itemOpen.Font = New-Object System.Drawing.Font($itemOpen.Font, [System.Drawing.FontStyle]::Bold)
$itemOpen.add_Click({
    Open-AppWindow
})

# 2. Wizard
$itemWizard = $menu.Items.Add("Мастер настройки микрофона...")
$itemWizard.add_Click({
    try {
        Invoke-RestMethod "$appUrl/api/wizard/trigger" -TimeoutSec 1 | Out-Null
    } catch {}
    Open-AppWindow "/#wizard"
})

$menu.Items.Add("-") | Out-Null

# 3. Stream Status Toggle
$itemStream = $menu.Items.Add("WASAPI аудиомост: [В ЭФИРЕ] (Пауза)")
$itemStream.add_Click({
    try {
        $st = Invoke-RestMethod "$appUrl/api/stream/status" -TimeoutSec 1
        if ($st.active) {
            Invoke-RestMethod "$appUrl/api/stream/stop" -TimeoutSec 1 | Out-Null
        } else {
            Invoke-RestMethod "$appUrl/api/stream/start" -TimeoutSec 1 | Out-Null
        }
    } catch {}
})

# 4. VB-Cable Installer
$itemCable = $menu.Items.Add("Установить VB-Audio Virtual Cable...")
$itemCable.add_Click({
    Start-Process $appExe -ArgumentList "vbcable"
})

# 5. Check Updates
$itemUpdates = $menu.Items.Add("Проверить обновления (GitHub)...")
$itemUpdates.add_Click({
    Start-Process "https://github.com/dimalinau-lab/EqualizerFD-Craft/releases"
})

$menu.Items.Add("-") | Out-Null

# 6. Autostart Toggle
$itemAuto = $menu.Items.Add("Автозагрузка при старте Windows")
$itemAuto.CheckOnClick = $true
$itemAuto.Checked = Get-AutostartStatus
$itemAuto.add_CheckedChanged({
    Set-AutostartStatus $itemAuto.Checked
})

# 7. Restart Audio Service
$itemRestart = $menu.Items.Add("Перезапустить аудиослужбу Windows")
$itemRestart.add_Click({
    try {
        Start-Process "powershell" -ArgumentList "-NoProfile -Command Restart-Service Audiosrv -Force" -Verb RunAs -WindowStyle Hidden
    } catch {}
})

$menu.Items.Add("-") | Out-Null

# 8. Exit
$itemExit = $menu.Items.Add("Выход из программы")
$itemExit.add_Click({
    $notify.Visible = $false
    $notify.Dispose()
    Stop-Process -Name "EqualizerFD-Craft" -Force -ErrorAction SilentlyContinue
    [System.Windows.Forms.Application]::Exit()
})

# Dynamic update on menu opening
$menu.add_Opening({
    try {
        $st = Invoke-RestMethod "$appUrl/api/stream/status" -TimeoutSec 1 -ErrorAction SilentlyContinue
        if ($st -and $st.active) {
            $itemStream.Text = "WASAPI аудиомост: [В ЭФИРЕ] (Нажмите для паузы)"
        } else {
            $itemStream.Text = "WASAPI аудиомост: [Пауза] (Нажмите для запуска)"
        }
    } catch {
        $itemStream.Text = "WASAPI аудиомост: [Не подключен]"
    }
    $itemAuto.Checked = Get-AutostartStatus
})

# Left click & Double click handlers
$notify.add_Click({
    param($sender, $e)
    if ($e.Button -eq [System.Windows.Forms.MouseButtons]::Left) {
        Open-AppWindow
    }
})
$notify.add_DoubleClick({
    Open-AppWindow
})

$notify.ContextMenuStrip = $menu
$notify.Visible = $true

# Watchdog timer: if EqualizerFD-Craft.exe is terminated, cleanly dispose tray icon and exit
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 2000
$timer.add_Tick({
    $proc = Get-Process -Name "EqualizerFD-Craft" -ErrorAction SilentlyContinue
    if (-not $proc) {
        $notify.Visible = $false
        $notify.Dispose()
        [System.Windows.Forms.Application]::Exit()
    }
})
$timer.Start()

# Run standard message loop
[System.Windows.Forms.Application]::Run()
