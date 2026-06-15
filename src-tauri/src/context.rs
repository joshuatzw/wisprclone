#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppContext {
    Code,
    Chat,
    Email,
    Terminal,
    #[default]
    General,
}

impl AppContext {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppContext::Code => "code",
            AppContext::Chat => "chat",
            AppContext::Email => "email",
            AppContext::Terminal => "terminal",
            AppContext::General => "general",
        }
    }
}

#[cfg(windows)]
pub fn detect_focused_app() -> AppContext {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return AppContext::General;
        }

        let mut title_buf = [0u16; 512];
        let title_len =
            GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        let title =
            String::from_utf16_lossy(&title_buf[..title_len.max(0) as usize]).to_lowercase();

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return AppContext::General;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return AppContext::General;
        }

        let mut path_buf = [0u16; 260];
        let mut path_len = 260u32;
        let ok =
            QueryFullProcessImageNameW(handle, 0, path_buf.as_mut_ptr(), &mut path_len);
        CloseHandle(handle);

        if ok == 0 {
            return AppContext::General;
        }

        let path = String::from_utf16_lossy(&path_buf[..path_len as usize]);
        let exe = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        match exe.as_str() {
            "code.exe"
            | "devenv.exe"
            | "idea64.exe"
            | "webstorm64.exe"
            | "pycharm64.exe"
            | "clion64.exe"
            | "rider64.exe"
            | "fleet.exe"
            | "sublime_text.exe"
            | "atom.exe"
            | "notepad++.exe"
            | "vim.exe"
            | "nvim.exe"
            | "emacs.exe"
            | "zed.exe"
            | "cursor.exe" => AppContext::Code,

            "windowsterminal.exe"
            | "wt.exe"
            | "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "alacritty.exe"
            | "wezterm.exe"
            | "mintty.exe"
            | "bash.exe"
            | "conhost.exe" => AppContext::Terminal,

            "slack.exe"
            | "discord.exe"
            | "mattermost.exe"
            | "signal.exe"
            | "telegram.exe"
            | "whatsapp.exe"
            | "teams.exe" => AppContext::Chat,

            "outlook.exe" | "thunderbird.exe" | "mailbird.exe" | "postbox.exe" => {
                AppContext::Email
            }

            "chrome.exe"
            | "firefox.exe"
            | "msedge.exe"
            | "brave.exe"
            | "opera.exe"
            | "vivaldi.exe"
            | "arc.exe"
            | "thorium.exe" => classify_browser(&title),

            _ => AppContext::General,
        }
    }
}

#[cfg(windows)]
fn classify_browser(title: &str) -> AppContext {
    if title.contains("gmail")
        || title.contains("yahoo mail")
        || title.contains("proton mail")
        || title.contains("compose")
        || (title.contains("outlook") && title.contains("mail"))
    {
        return AppContext::Email;
    }
    if title.contains("slack")
        || title.contains("discord")
        || title.contains("teams")
        || title.contains("whatsapp")
        || title.contains("telegram")
        || title.contains("messenger")
    {
        return AppContext::Chat;
    }
    AppContext::General
}

#[cfg(not(windows))]
pub fn detect_focused_app() -> AppContext {
    AppContext::General
}
