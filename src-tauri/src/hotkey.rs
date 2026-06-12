// Low-level keyboard hook to capture Ctrl+Win push-to-talk.
// Recording starts when Win is pressed while Ctrl is held.
// Recording stops when EITHER key is released — this handles any release order.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LWIN, VK_RCONTROL, VK_RWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    KBDLLHOOKSTRUCT, MSG,
};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};

static TX: OnceLock<mpsc::SyncSender<bool>> = OnceLock::new();
static WIN_DOWN: AtomicBool = AtomicBool::new(false);

pub fn start(tx: mpsc::SyncSender<bool>) {
    TX.set(tx).expect("hotkey already started");
    std::thread::spawn(|| unsafe {
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), 0, 0);
        if hook == 0 {
            eprintln!("[wispr] Failed to install keyboard hook");
            return;
        }
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let r = GetMessageW(&mut msg, 0, 0, 0);
            if r <= 0 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(hook);
    });
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;

        let is_win = vk == VK_LWIN as u32 || vk == VK_RWIN as u32;
        let is_ctrl_key = vk == VK_LCONTROL as u32 || vk == VK_RCONTROL as u32;
        let is_down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let is_up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);

        // Start: Win key pressed while Ctrl is held
        if is_win && is_down {
            let ctrl_held = GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0;
            if ctrl_held {
                if WIN_DOWN
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    if let Some(tx) = TX.get() {
                        tx.try_send(true).ok();
                    }
                }
                return 1; // suppress Win key → prevents Start menu
            }
        }

        // Stop: Win key OR Ctrl key released while we are recording.
        // This handles both release orders: Win-first or Ctrl-first.
        if is_up && (is_win || is_ctrl_key) && WIN_DOWN.load(Ordering::SeqCst) {
            if WIN_DOWN
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                if let Some(tx) = TX.get() {
                    tx.try_send(false).ok();
                }
            }
            if is_win {
                return 1; // suppress Win key release too
            }
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}
