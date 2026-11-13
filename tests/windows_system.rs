#![cfg(target_os = "windows")]

/// Changes only FreePinPaste's own Run value and restores it, even on assertion failure.
#[test]
#[ignore = "temporarily changes the current user's FreePinPaste autostart registration"]
fn user_autostart_roundtrip() {
    use freepinpaste::config;
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let key = windows_registry::CURRENT_USER.create(RUN).unwrap();
            if let Some(value) = &self.0 {
                key.set_string("FreePinPaste", value).unwrap();
            } else {
                let _ = key.remove_value("FreePinPaste");
            }
        }
    }
    let key = windows_registry::CURRENT_USER.create(RUN).unwrap();
    let _restore = Restore(key.get_string("FreePinPaste").ok());
    config::set_autostart(true).unwrap();
    assert!(config::autostart_enabled());
    assert_eq!(
        key.get_string("FreePinPaste").unwrap(),
        format!(
            "\"{}\" --background",
            std::env::current_exe().unwrap().display()
        )
    );
    config::set_autostart(false).unwrap();
    assert!(!config::autostart_enabled());
    config::set_autostart(false).unwrap();
}
