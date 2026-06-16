// Low-level keyboard hook for push-to-talk.
// The active combo is stored in COMBO (AtomicU8):
//   0 = Ctrl+Win  1 = Right Alt  2 = Ctrl+Shift  3 = Ctrl+Alt
//
// Ctrl+Win: Win pressed while Ctrl held; Win key is suppressed (no Start menu).
// Right Alt: held alone (AltGr excluded by checking Ctrl is NOT held).
// Ctrl+Shift: Shift pressed while Ctrl held; either key released stops recording.
// Ctrl+Alt: Alt pressed while Ctrl held; either key released stops recording.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, OnceLock};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL,
    VK_RMENU, VK_RSHIFT, VK_RWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    KBDLLHOOKSTRUCT, MSG,
};

static TX: OnceLock<mpsc::SyncSender<bool>> = OnceLock::new();
static WIN_DOWN: AtomicBool = AtomicBool::new(false);
static COMBO: AtomicU8 = AtomicU8::new(0);

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

pub fn set_combo(combo: u8) {
    COMBO.store(combo, Ordering::SeqCst);
}

pub fn reset() {
    WIN_DOWN.store(false, Ordering::SeqCst);
}

fn send_press() {
    if WIN_DOWN
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        if let Some(tx) = TX.get() {
            tx.try_send(true).ok();
        }
    }
}

fn send_release() {
    if WIN_DOWN
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        if let Some(tx) = TX.get() {
            tx.try_send(false).ok();
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;
        let is_down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let is_up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
        let ctrl_held = || GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000 != 0;

        match COMBO.load(Ordering::Relaxed) {
            0 => {
                // Ctrl+Win
                let is_win = vk == VK_LWIN as u32 || vk == VK_RWIN as u32;
                let is_ctrl = vk == VK_LCONTROL as u32 || vk == VK_RCONTROL as u32;
                if is_down && is_win && ctrl_held() {
                    send_press();
                    return 1; // suppress Win key → no Start menu
                }
                if is_up && (is_win || is_ctrl) && WIN_DOWN.load(Ordering::SeqCst) {
                    send_release();
                    if is_win {
                        return 1; // suppress Win key release too
                    }
                }
            }
            1 => {
                // Right Alt held alone (AltGr on European layouts synthesises LCtrl+RAlt — exclude it)
                let is_ralt = vk == VK_RMENU as u32;
                if is_down && is_ralt && !ctrl_held() {
                    send_press();
                }
                if is_up && is_ralt {
                    send_release();
                }
            }
            2 => {
                // Ctrl+Shift
                let is_shift = vk == VK_LSHIFT as u32 || vk == VK_RSHIFT as u32;
                let is_ctrl = vk == VK_LCONTROL as u32 || vk == VK_RCONTROL as u32;
                if is_down && is_shift && ctrl_held() {
                    send_press();
                }
                if is_up && (is_shift || is_ctrl) {
                    send_release();
                }
            }
            3 => {
                // Ctrl+Alt
                let is_alt = vk == VK_LMENU as u32 || vk == VK_RMENU as u32;
                let is_ctrl = vk == VK_LCONTROL as u32 || vk == VK_RCONTROL as u32;
                if is_down && is_alt && ctrl_held() {
                    send_press();
                }
                if is_up && (is_alt || is_ctrl) {
                    send_release();
                }
            }
            _ => {}
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}
