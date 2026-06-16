/// Halve the volume of all audio render sessions (called when recording starts).
pub fn duck() {
    #[cfg(windows)]
    scale_sessions(0.5);
}

/// Restore audio render sessions to their pre-duck levels (called when recording stops).
pub fn unduck() {
    #[cfg(windows)]
    scale_sessions(2.0);
}

/// Multiply every render session's per-session volume by `factor`, clamped to [0, 1].
/// duck(0.5) then unduck(2.0) is a perfect round-trip provided nobody adjusts
/// their volume in between — acceptable for a push-to-talk use case.
#[cfg(windows)]
fn scale_sessions(factor: f32) {
    use windows::{
        core::Interface,
        Win32::{
            Media::Audio::{
                eConsole, eRender, IAudioSessionManager2, IMMDeviceEnumerator,
                ISimpleAudioVolume, MMDeviceEnumerator,
            },
            System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
        },
    };

    unsafe {
        // S_FALSE means already initialised on this thread — that is fine.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let device_enum: IMMDeviceEnumerator =
            match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(e) => { eprintln!("[wispr] volume: CoCreateInstance: {e}"); return; }
            };

        let device = match device_enum.GetDefaultAudioEndpoint(eRender, eConsole) {
            Ok(d) => d,
            Err(e) => { eprintln!("[wispr] volume: GetDefaultAudioEndpoint: {e}"); return; }
        };

        let mgr: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
            Ok(m) => m,
            Err(e) => { eprintln!("[wispr] volume: Activate IAudioSessionManager2: {e}"); return; }
        };

        let session_enum = match mgr.GetSessionEnumerator() {
            Ok(e) => e,
            Err(e) => { eprintln!("[wispr] volume: GetSessionEnumerator: {e}"); return; }
        };

        let count = match session_enum.GetCount() {
            Ok(c) => c,
            Err(_) => return,
        };

        for i in 0..count {
            let Ok(session) = session_enum.GetSession(i) else { continue };
            let Ok(vol) = session.cast::<ISimpleAudioVolume>() else { continue };
            let Ok(current) = vol.GetMasterVolume() else { continue };
            let _ = vol.SetMasterVolume((current * factor).clamp(0.0, 1.0), std::ptr::null());
        }
    }
}
