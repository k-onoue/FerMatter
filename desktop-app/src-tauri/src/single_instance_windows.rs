// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// Adapted from tauri-plugin-single-instance 2.5.4 (Apache-2.0 OR MIT).
// Keep the pre-migration mutex/window names so old and new apps cannot write concurrently.
// https://github.com/tauri-apps/plugins-workspace/tree/single-instance-v2.5.4/plugins/single-instance

type SingleInstanceCallback<R> = dyn FnMut(&AppHandle<R>, Vec<String>, String) + Send + Sync;
use tauri::{
    plugin::{self, TauriPlugin},
    AppHandle, Manager, RunEvent, Runtime,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM},
    System::{
        DataExchange::COPYDATASTRUCT,
        LibraryLoader::GetModuleHandleW,
        Threading::{CreateMutexW, ReleaseMutex},
    },
    UI::WindowsAndMessaging::{
        self as w32wm, AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DestroyWindow,
        FindWindowW, GetWindowThreadProcessId, RegisterClassExW, SendMessageTimeoutW,
        CREATESTRUCTW, GWLP_USERDATA, GWL_STYLE, SMTO_NORMAL, WINDOW_LONG_PTR_INDEX, WM_COPYDATA,
        WM_CREATE, WM_DESTROY, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
    },
};

// TODO(v3): remove this
/// Legacy payload: cwd and arguments joined by `|`, which splits any argument containing a `|`.
/// Still accepted from second instances running an older version of the plugin.
const WMCOPYDATA_SINGLE_INSTANCE_DATA: usize = 1542;
/// Cwd and arguments separated by NUL, without a terminator.
const WMCOPYDATA_SINGLE_INSTANCE_DATA_NUL: usize = 1543;

/// How long the second instance waits for the first instance to take its arguments.
const FORWARD_TIMEOUT_MS: u32 = 10_000;

struct MutexHandle(isize);

struct TargetWindowHandle(isize);

struct UserData<R: Runtime> {
    app: AppHandle<R>,
    callback: Box<SingleInstanceCallback<R>>,
}

impl<R: Runtime> UserData<R> {
    unsafe fn from_hwnd_raw(hwnd: HWND) -> *mut Self {
        unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self }
    }

    unsafe fn from_hwnd<'a>(hwnd: HWND) -> &'a mut Self {
        unsafe { &mut *Self::from_hwnd_raw(hwnd) }
    }

    fn run_callback(&mut self, args: Vec<String>, cwd: String) {
        (self.callback)(&self.app, args, cwd)
    }
}

pub fn init<R: Runtime>(callback: Box<SingleInstanceCallback<R>>) -> TauriPlugin<R> {
    plugin::Builder::new("single-instance")
        .setup(|app, _api| {
            let id = crate::identity::LEGACY_ID;

            let class_name = encode_wide(format!("{id}-sic"));
            let window_name = encode_wide(format!("{id}-siw"));
            let mutex_name = encode_wide(format!("{id}-sim"));

            let hmutex =
                unsafe { CreateMutexW(std::ptr::null(), true.into(), mutex_name.as_ptr()) };

            let last_error = unsafe { GetLastError() };
            if hmutex.is_null() {
                return Err(std::io::Error::from_raw_os_error(last_error as i32).into());
            }
            if last_error == ERROR_ALREADY_EXISTS {
                unsafe {
                    // The first instance may still be creating its forwarding window.
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    let hwnd = loop {
                        let hwnd = FindWindowW(class_name.as_ptr(), window_name.as_ptr());
                        if !hwnd.is_null() || std::time::Instant::now() >= deadline { break hwnd; }
                        std::thread::sleep(std::time::Duration::from_millis(40));
                    };
                    CloseHandle(hmutex);

                    if !hwnd.is_null() {
                        // Windows lets us bring a window to the front, but not the first
                        // instance. Hand that right over before we exit, so focusing a window
                        // from the callback works. Windows takes it back if the user switches
                        // to another app in the meantime.
                        let mut pid = 0;
                        GetWindowThreadProcessId(hwnd, &mut pid);
                        if pid != 0 {
                            AllowSetForegroundWindow(pid);
                        }

                        // Older previews understand only the pipe-delimited protocol.
                        // Windows file names cannot contain a pipe; our launch payload is SVG paths.
                        let cwd = std::env::current_dir().unwrap_or_default();
                        let args = std::env::args_os().map(|s| s.to_string_lossy().into_owned()).collect::<Vec<_>>();
                        let data = format!("{}|{}\0", cwd.display(), args.join("|"));

                        let bytes = data.as_bytes();
                        let cds = COPYDATASTRUCT {
                            dwData: WMCOPYDATA_SINGLE_INSTANCE_DATA,
                            cbData: bytes.len() as _,
                            lpData: bytes.as_ptr() as _,
                        };

                        let sent = SendMessageTimeoutW(
                            hwnd,
                            WM_COPYDATA,
                            0,
                            &cds as *const _ as _,
                            SMTO_NORMAL,
                            FORWARD_TIMEOUT_MS,
                            std::ptr::null_mut(),
                        );
                        if sent == 0 {
                            return Err(std::io::Error::other("The running FerMatter did not accept the launch. Quit it before retrying.").into());
                        }

                        app.cleanup_before_exit();
                        std::process::exit(0);
                    }
                    return Err(std::io::Error::other("FerMatter is starting or quitting. Retry after it finishes.").into());
                }
            } else {
                let userdata = UserData {
                    app: app.clone(),
                    callback,
                };
                let userdata = Box::into_raw(Box::new(userdata));
                let hwnd = create_event_target_window::<R>(&class_name, &window_name, userdata);
                if hwnd.is_null() {
                    unsafe { drop(Box::from_raw(userdata)); ReleaseMutex(hmutex); CloseHandle(hmutex); }
                    return Err(std::io::Error::last_os_error().into());
                }
                app.manage(MutexHandle(hmutex as _));
                app.manage(TargetWindowHandle(hwnd as _));
            }

            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::Exit = event {
                destroy(app);
            }
        })
        .build()
}

pub fn destroy<R: Runtime, M: Manager<R>>(manager: &M) {
    if let Some(hmutex) = manager.try_state::<MutexHandle>() {
        unsafe {
            ReleaseMutex(hmutex.0 as _);
            CloseHandle(hmutex.0 as _);
        }
    }
    if let Some(hwnd) = manager.try_state::<TargetWindowHandle>() {
        unsafe { DestroyWindow(hwnd.0 as _) };
    }
}

unsafe extern "system" fn single_instance_window_proc<R: Runtime>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = unsafe { &*(lparam as *const CREATESTRUCTW) };
            let userdata = create_struct.lpCreateParams as *const UserData<R>;
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, userdata as _) };
            0
        }

        WM_COPYDATA => {
            let cds = unsafe { &*(lparam as *const COPYDATASTRUCT) };
            let separator = match cds.dwData {
                WMCOPYDATA_SINGLE_INSTANCE_DATA_NUL => '\0',
                WMCOPYDATA_SINGLE_INSTANCE_DATA => '|',
                _ => return 1,
            };

            if cds.cbData > 64 * 1024 {
                return 0;
            }
            let bytes = if cds.lpData.is_null() {
                &[][..]
            } else {
                unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, cds.cbData as _) }
            };
            // The legacy format ends with a NUL terminator. The new one doesn't: a trailing NUL
            // there precedes an empty last argument.
            let bytes = if separator == '|' {
                bytes.split(|b| *b == 0).next().unwrap_or_default()
            } else {
                bytes
            };
            let data = String::from_utf8_lossy(bytes);

            let mut s = data.split(separator);
            let cwd = s.next().unwrap_or_default().to_string();
            let args = s.map(|s| s.to_string()).collect();

            let userdata = unsafe { UserData::<R>::from_hwnd(hwnd) };
            userdata.run_callback(args, cwd);
            1
        }

        WM_DESTROY => {
            let userdata = unsafe { UserData::<R>::from_hwnd_raw(hwnd) };
            drop(unsafe { Box::from_raw(userdata) });
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn create_event_target_window<R: Runtime>(
    class_name: &[u16],
    window_name: &[u16],
    userdata: *const UserData<R>,
) -> HWND {
    unsafe {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(single_instance_window_proc::<R>),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };

        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE
            | WS_EX_TRANSPARENT
            | WS_EX_LAYERED
            // WS_EX_TOOLWINDOW prevents this window from ever showing up in the taskbar, which
            // we want to avoid. If you remove this style, this window won't show up in the
            // taskbar *initially*, but it can show up at some later point. This can sometimes
            // happen on its own after several hours have passed, although this has proven
            // difficult to reproduce. Alternatively, it can be manually triggered by killing
            // `explorer.exe` and then starting the process back up.
            // It is unclear why the bug is triggered by waiting for several hours.
            | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            userdata as _,
        );
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            // The window technically has to be visible to receive WM_PAINT messages (which are used
            // for delivering events during resizes), but it isn't displayed to the user because of
            // the LAYERED style.
            (WS_VISIBLE | WS_POPUP) as isize,
        );
        hwnd
    }
}

pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_pointer_width = "32")]
#[allow(non_snake_case)]
unsafe fn SetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    unsafe { w32wm::SetWindowLongW(hwnd, index, value as _) as _ }
}

#[cfg(target_pointer_width = "64")]
#[allow(non_snake_case)]
unsafe fn SetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    unsafe { w32wm::SetWindowLongPtrW(hwnd, index, value) }
}

#[cfg(target_pointer_width = "32")]
#[allow(non_snake_case)]
unsafe fn GetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { w32wm::GetWindowLongW(hwnd, index) as _ }
}

#[cfg(target_pointer_width = "64")]
#[allow(non_snake_case)]
unsafe fn GetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    unsafe { w32wm::GetWindowLongPtrW(hwnd, index) }
}
