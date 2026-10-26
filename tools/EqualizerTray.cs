using System;
using System.Drawing;
using System.Windows.Forms;
using System.IO;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Net;
using System.Threading;
using Microsoft.Win32;

class EqualizerTrayApp {
    [DllImport("user32.dll")]
    static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")]
    static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")]
    static extern bool IsWindowVisible(IntPtr hWnd);

    delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    const int SW_RESTORE = 9;
    const int SW_SHOW = 5;
    const string APP_MUTEX_NAME = "EqualizerFDCraftTrayMutex";
    const string REG_RUN_KEY = @"Software\Microsoft\Windows\CurrentVersion\Run";
    const string REG_APP_NAME = "EqualizerFDCraft";

    static NotifyIcon trayIcon;
    static MenuItem streamMenuItem;
    static MenuItem autoMenuItem;
    static int parentPid = -1;

    [STAThread]
    static void Main(string[] args) {
        bool createdNew;
        using (Mutex mutex = new Mutex(true, APP_MUTEX_NAME, out createdNew)) {
            if (!createdNew) {
                FocusOrOpenApp();
                return;
            }

            for (int i = 0; i < args.Length; i++) {
                if (args[i].StartsWith("/parent:")) {
                    int.TryParse(args[i].Substring(8), out parentPid);
                }
            }

            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            trayIcon = new NotifyIcon();
            trayIcon.Text = "Equalizer FD CRAFT - Studio Audio Engine";
            trayIcon.Icon = LoadAppIcon();
            trayIcon.Visible = true;

            ContextMenu menu = new ContextMenu();

            MenuItem openItem = new MenuItem("Открыть Equalizer FD CRAFT", (s, e) => FocusOrOpenApp());
            openItem.DefaultItem = true;
            menu.MenuItems.Add(openItem);

            menu.MenuItems.Add("-");

            streamMenuItem = new MenuItem("WASAPI аудиомост: [Проверка...]", (s, e) => ToggleStream());
            menu.MenuItems.Add(streamMenuItem);

            autoMenuItem = new MenuItem("Автозагрузка с Windows", (s, e) => ToggleAutostart());
            autoMenuItem.Checked = IsAutostartEnabled();
            menu.MenuItems.Add(autoMenuItem);

            MenuItem restartAudioItem = new MenuItem("Перезапустить аудиослужбу (audiodg.exe)", (s, e) => RestartAudioService());
            menu.MenuItems.Add(restartAudioItem);

            MenuItem folderItem = new MenuItem("Открыть папку приложения", (s, e) => OpenAppFolder());
            menu.MenuItems.Add(folderItem);

            menu.MenuItems.Add("-");

            MenuItem exitItem = new MenuItem("Выход", (s, e) => ExitApplication());
            menu.MenuItems.Add(exitItem);

            trayIcon.ContextMenu = menu;
            trayIcon.DoubleClick += (s, e) => FocusOrOpenApp();

            // Periodic status and watchdog timer
            System.Windows.Forms.Timer watchdog = new System.Windows.Forms.Timer();
            watchdog.Interval = 2500;
            watchdog.Tick += (s, e) => {
                CheckParentProcess();
                UpdateStreamStatus();
            };
            watchdog.Start();

            UpdateStreamStatus();

            Application.Run();

            if (trayIcon != null) {
                trayIcon.Visible = false;
                trayIcon.Dispose();
            }
        }
    }

    static Icon LoadAppIcon() {
        string[] candidates = new string[] {
            Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "EqualizerFD-Craft.ico"),
            Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "icon.ico"),
            @"D:\testik\EqualizerFD-Craft.ico",
            @"D:\testik\icon.ico"
        };

        foreach (string path in candidates) {
            if (File.Exists(path)) {
                try {
                    return new Icon(path, 32, 32);
                } catch {
                    try {
                        return new Icon(path);
                    } catch { }
                }
            }
        }
        return SystemIcons.Application;
    }

    static void FocusOrOpenApp() {
        bool httpSuccess = false;
        try {
            HttpWebRequest req = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:49200/api/window/show");
            req.Method = "POST";
            req.ContentLength = 0;
            req.Timeout = 1500;
            using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse()) {
                if (resp.StatusCode == HttpStatusCode.OK) {
                    httpSuccess = true;
                }
            }
        } catch { }

        if (httpSuccess) {
            return;
        }

        IntPtr targetHwnd = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            System.Text.StringBuilder sb = new System.Text.StringBuilder(512);
            if (GetWindowText(hWnd, sb, 512) > 0) {
                string title = sb.ToString();
                if (title.IndexOf("Equalizer FD CRAFT", StringComparison.OrdinalIgnoreCase) >= 0) {
                    targetHwnd = hWnd;
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);

        if (targetHwnd != IntPtr.Zero) {
            ShowWindow(targetHwnd, SW_SHOW);
            ShowWindow(targetHwnd, SW_RESTORE);
            BringWindowToTop(targetHwnd);
            SetForegroundWindow(targetHwnd);
        } else {
            try {
                string exePath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "EqualizerFD-Craft.exe");
                if (!File.Exists(exePath)) {
                    exePath = @"D:\testik\EqualizerFD-Craft.exe";
                }
                if (File.Exists(exePath)) {
                    ProcessStartInfo psi = new ProcessStartInfo(exePath, "editor");
                    psi.UseShellExecute = false;
                    psi.CreateNoWindow = true;
                    Process.Start(psi);
                }
            } catch { }
        }
    }

    static void CheckParentProcess() {
        if (parentPid > 0) {
            try {
                Process p = Process.GetProcessById(parentPid);
                if (p.HasExited) {
                    Application.Exit();
                }
            } catch {
                Application.Exit();
            }
        } else {
            Process[] procs = Process.GetProcessesByName("EqualizerFD-Craft");
            if (procs.Length == 0) {
                Application.Exit();
            }
        }
    }

    static void UpdateStreamStatus() {
        ThreadPool.QueueUserWorkItem((state) => {
            try {
                HttpWebRequest req = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:49200/api/status");
                req.Timeout = 1200;
                using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse())
                using (StreamReader reader = new StreamReader(resp.GetResponseStream())) {
                    string json = reader.ReadToEnd();
                    bool active = json.IndexOf("\"stream_active\":true", StringComparison.OrdinalIgnoreCase) >= 0
                               || json.IndexOf("\"active\":true", StringComparison.OrdinalIgnoreCase) >= 0;
                    if (streamMenuItem != null) {
                        streamMenuItem.Text = active
                            ? "WASAPI аудиомост: [В ЭФИРЕ] (клик для паузы)"
                            : "WASAPI аудиомост: [Пауза] (клик для запуска)";
                    }
                }
            } catch {
                if (streamMenuItem != null) {
                    streamMenuItem.Text = "WASAPI аудиомост: [Пауза] (клик для запуска)";
                }
            }
        });
    }

    static void ToggleStream() {
        ThreadPool.QueueUserWorkItem((state) => {
            try {
                HttpWebRequest req = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:49200/api/stream/toggle");
                req.Method = "POST";
                req.ContentLength = 0;
                req.Timeout = 2000;
                using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse()) { }
            } catch { }
            Thread.Sleep(200);
            UpdateStreamStatus();
        });
    }

    static bool IsAutostartEnabled() {
        try {
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(REG_RUN_KEY, false)) {
                if (key != null) {
                    object val = key.GetValue(REG_APP_NAME);
                    return val != null;
                }
            }
        } catch { }
        return false;
    }

    static void ToggleAutostart() {
        try {
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(REG_RUN_KEY, true)) {
                if (key != null) {
                    bool enabled = IsAutostartEnabled();
                    if (enabled) {
                        key.DeleteValue(REG_APP_NAME, false);
                        autoMenuItem.Checked = false;
                    } else {
                        string exePath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "EqualizerFD-Craft.exe");
                        if (!File.Exists(exePath)) {
                            exePath = @"D:\testik\EqualizerFD-Craft.exe";
                        }
                        key.SetValue(REG_APP_NAME, "\"" + exePath + "\" gui");
                        autoMenuItem.Checked = true;
                    }
                }
            }
        } catch { }
    }

    static void RestartAudioService() {
        ThreadPool.QueueUserWorkItem((state) => {
            // First try internal server API (elevated without UAC prompt)
            try {
                HttpWebRequest req = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:49200/api/apo/restart-engine");
                req.Method = "POST";
                req.ContentLength = 0;
                req.Timeout = 3000;
                using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse()) {
                    if (resp.StatusCode == HttpStatusCode.OK) {
                        if (trayIcon != null) {
                            trayIcon.ShowBalloonTip(2000, "Equalizer FD CRAFT", "Аудиослужба (audiodg.exe) успешно перезапущена", ToolTipIcon.Info);
                        }
                        return;
                    }
                }
            } catch { }

            // Fallback to elevated cmd
            try {
                ProcessStartInfo psi = new ProcessStartInfo();
                psi.FileName = "cmd.exe";
                psi.Arguments = "/c net stop audiosrv /y && net start audiosrv";
                psi.CreateNoWindow = true;
                psi.UseShellExecute = true; // Required for Verb = "runas"
                psi.Verb = "runas";
                Process.Start(psi);
            } catch { }
        });
    }

    static void OpenAppFolder() {
        try {
            string baseDir = AppDomain.CurrentDomain.BaseDirectory.TrimEnd('\\');
            Process.Start("explorer.exe", "\"" + baseDir + "\"");
        } catch { }
    }

    static void ExitApplication() {
        try {
            if (trayIcon != null) {
                trayIcon.Visible = false;
                trayIcon.Dispose();
            }
        } catch { }

        try {
            HttpWebRequest req = (HttpWebRequest)WebRequest.Create("http://127.0.0.1:49200/api/app/quit");
            req.Method = "POST";
            req.ContentLength = 0;
            req.Timeout = 1200;
            using (HttpWebResponse resp = (HttpWebResponse)req.GetResponse()) { }
        } catch {
            try {
                Process[] procs = Process.GetProcessesByName("EqualizerFD-Craft");
                foreach (Process p in procs) {
                    try { p.Kill(); } catch { }
                }
            } catch { }
        }
        Application.Exit();
    }
}
