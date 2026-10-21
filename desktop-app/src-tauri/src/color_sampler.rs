//! AppKit owns the sampling UI; no screenshot files or screen-recording access.
use crate::files::Result;

#[cfg(target_os = "macos")]
pub async fn sample(app: &tauri::AppHandle) -> Result<Option<String>> {
    use block2::RcBlock;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};
    use std::{
        cell::RefCell,
        sync::atomic::{AtomicBool, Ordering},
    };
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return Err("Finish the current color selection first.".into());
    }
    struct Sampling;
    impl Drop for Sampling {
        fn drop(&mut self) {
            ACTIVE.store(false, Ordering::SeqCst);
        }
    }
    let _sampling = Sampling;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let sender = RefCell::new(Some(sender));
        let handler = RcBlock::new(move |color: *mut NSColor| {
            // AppKit supplies a borrowed color only for this callback, or nil
            // on Escape. Convert to the same opaque sRGB HEX used by exports.
            let result = unsafe { color.as_ref() }
                .map(|color| {
                    color
                        .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
                        .map(|rgb| {
                            hex([
                                rgb.redComponent(),
                                rgb.greenComponent(),
                                rgb.blueComponent(),
                            ])
                        })
                        .ok_or_else(|| "Could not convert the selected color to sRGB.".to_owned())
                })
                .transpose();
            if let Some(sender) = sender.borrow_mut().take() {
                let _ = sender.send(result);
            }
        });
        // AppKit retains the sampler and copies its handler until selection or
        // cancellation. Never wait on the main thread for that callback.
        unsafe {
            NSColorSampler::new().showSamplerWithSelectionHandler(&handler);
        }
    })
    .map_err(crate::files::err)?;
    receiver.await.map_err(crate::files::err)?
}

#[cfg(not(target_os = "macos"))]
pub async fn sample(_: &tauri::AppHandle) -> Result<Option<String>> {
    Err("Screen color sampling is currently available on macOS.".into())
}

#[cfg(any(target_os = "macos", test))]
fn hex(components: [f64; 3]) -> String {
    let [r, g, b] = components.map(|value| (value.clamp(0., 1.) * 255.).round() as u8);
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn sampled_components_round_and_clamp_to_srgb_hex() {
        assert_eq!(super::hex([0., 0.5, 1.]), "#0080ff");
        assert_eq!(super::hex([-0.1, 0.25, 1.1]), "#0040ff");
    }
}
