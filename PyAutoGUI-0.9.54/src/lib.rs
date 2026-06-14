use pyo3::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static FAILSAFE_TRIGGERED: AtomicBool = AtomicBool::new(false);
static HOOK_ONCE: Once = Once::new();

#[cfg(target_os = "windows")]
mod win {
    use pyo3::prelude::*;
    use pyo3::types::PyTuple;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SetProcessDPIAware,
        SM_CXSCREEN, SM_CYSCREEN, SM_SWAPBUTTON, GetMessageExtraInfo,
        SetWindowsHookExA, UnhookWindowsHookEx, CallNextHookEx,
        GetMessageA, TranslateMessage, DispatchMessageA,
        MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL
    };
    use windows_sys::Win32::Graphics::Gdi::{
        GetDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject,
        BitBlt, GetDIBits, DeleteObject, DeleteDC, ReleaseDC,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY
    };
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, MOUSEINPUT, KEYBDINPUT, INPUT_MOUSE, INPUT_KEYBOARD,
        MapVirtualKeyA, KEYEVENTF_SCANCODE, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE, VkKeyScanA
    };
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::Foundation::POINT;

    #[link(name = "winmm")]
    extern "system" {
        fn timeBeginPeriod(uPeriod: u32) -> u32;
        fn timeEndPeriod(uPeriod: u32) -> u32;
    }

    #[repr(C)]
    struct InputEvent {
        event_type: u32,
        mouse_flags: u16,
        button_flags: u16,
        x: i32,
        y: i32,
        keyboard_flags: u32,
    }

    extern "system" {
        fn CreateFileA(
            lpFileName: *const u8,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        fn DeviceIoControl(
            hDevice: isize,
            dwIoControlCode: u32,
            lpInBuffer: *const std::ffi::c_void,
            nInBufferSize: u32,
            lpOutBuffer: *mut std::ffi::c_void,
            nOutBufferSize: u32,
            lpBytesReturned: *mut u32,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
    }

    static mut FORCE_DISABLE_DRIVER: bool = false;
    static DRIVER_HANDLE: std::sync::OnceLock<Option<isize>> = std::sync::OnceLock::new();

    fn get_driver_handle() -> Option<isize> {
        unsafe {
            if FORCE_DISABLE_DRIVER {
                return None;
            }
        }
        *DRIVER_HANDLE.get_or_init(|| {
            unsafe {
                let handle = CreateFileA(
                    b"\\\\.\\MyDriver\0".as_ptr(),
                    0x80000000 | 0x40000000,
                    1 | 2,
                    std::ptr::null(),
                    3,
                    0x80,
                    0
                );
                if handle == -1 {
                    None
                } else {
                    Some(handle)
                }
            }
        })
    }

    #[pyfunction]
    pub fn set_use_driver(use_driver: bool) -> PyResult<()> {
        unsafe {
            FORCE_DISABLE_DRIVER = !use_driver;
        }
        Ok(())
    }

    fn try_send_driver_keyboard(vk: u8, scan: u8, flags: u32) -> bool {
        if let Some(handle) = get_driver_handle() {
            let event = InputEvent {
                event_type: 1,
                mouse_flags: 0,
                button_flags: 0,
                x: vk as i32,
                y: scan as i32,
                keyboard_flags: flags,
            };
            let mut returned = 0u32;
            let res = unsafe {
                DeviceIoControl(
                    handle,
                    0x00222000,
                    &event as *const InputEvent as *const std::ffi::c_void,
                    std::mem::size_of::<InputEvent>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut()
                )
            };
            res != 0
        } else {
            false
        }
    }

    fn try_send_driver_mouse(mouse_flags: u16, button_flags: u16, x: i32, y: i32) -> bool {
        if let Some(handle) = get_driver_handle() {
            let event = InputEvent {
                event_type: 0,
                mouse_flags,
                button_flags,
                x,
                y,
                keyboard_flags: 0,
            };
            let mut returned = 0u32;
            let res = unsafe {
                DeviceIoControl(
                    handle,
                    0x00222000,
                    &event as *const InputEvent as *const std::ffi::c_void,
                    std::mem::size_of::<InputEvent>() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut()
                )
            };
            res != 0
        } else {
            false
        }
    }

    #[pyfunction]
    pub fn set_process_dpi_aware() -> PyResult<bool> {
        unsafe {
            timeBeginPeriod(1);
        }
        let result = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if result != 0 {
            Ok(true)
        } else {
            let legacy_result = unsafe { SetProcessDPIAware() };
            Ok(legacy_result != 0)
        }
    }

    #[pyfunction]
    pub fn time_begin_period(period: u32) -> PyResult<u32> {
        let res = unsafe { timeBeginPeriod(period) };
        Ok(res)
    }

    #[pyfunction]
    pub fn time_end_period(period: u32) -> PyResult<u32> {
        let res = unsafe { timeEndPeriod(period) };
        Ok(res)
    }

    #[pyfunction]
    pub fn get_cursor_pos() -> PyResult<(i32, i32)> {
        let mut point = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut point) };
        if ok != 0 {
            Ok((point.x, point.y))
        } else {
            Err(pyo3::exceptions::PyOSError::new_err("Failed to get cursor position"))
        }
    }

    #[pyfunction]
    pub fn get_system_metrics(index: i32) -> PyResult<i32> {
        let val = unsafe { GetSystemMetrics(index) };
        Ok(val)
    }

    #[pyfunction]
    pub fn set_cursor_pos(x: i32, y: i32) -> PyResult<()> {
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if width == 0 || height == 0 {
            return Err(pyo3::exceptions::PyOSError::new_err("Failed to get screen metrics"));
        }
        
        // Convert to absolute normalized coordinates (0 to 65535)
        let dx = (x * 65535) / (width - 1);
        let dy = (y * 65535) / (height - 1);

        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx,
            dy,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
            time: unsafe { GetTickCount() },
            dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
        };

        let sent = unsafe {
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32)
        };
        if sent == 1 {
            Ok(())
        } else {
            Err(pyo3::exceptions::PyOSError::new_err("Failed to set cursor position via SendInput"))
        }
    }

    #[pyfunction]
    pub fn send_mouse_event(ev: u32, x: i32, y: i32, data: i32) -> PyResult<()> {
        let mut button_flags = 0u16;
        if (ev & 0x0002) != 0 { button_flags |= 0x0001; }
        if (ev & 0x0004) != 0 { button_flags |= 0x0002; }
        if (ev & 0x0008) != 0 { button_flags |= 0x0004; }
        if (ev & 0x0010) != 0 { button_flags |= 0x0008; }
        if (ev & 0x0020) != 0 { button_flags |= 0x0010; }
        if (ev & 0x0040) != 0 { button_flags |= 0x0020; }
        if (ev & 0x0800) != 0 { button_flags |= 0x0400; }

        let mouse_flags = if (ev & 0x8000) != 0 { 1 } else { 0 };

        if try_send_driver_mouse(mouse_flags, button_flags, x, y) {
            return Ok(());
        }

        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if width == 0 || height == 0 {
            return Err(pyo3::exceptions::PyOSError::new_err("Failed to get screen metrics"));
        }
        
        let converted_x = (x * 65535) / (width - 1);
        let converted_y = (y * 65535) / (height - 1);

        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx: converted_x,
            dy: converted_y,
            mouseData: data as u32,
            dwFlags: ev | MOUSEEVENTF_ABSOLUTE,
            time: unsafe { GetTickCount() },
            dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
        };

        let sent = unsafe {
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32)
        };
        if sent == 1 {
            Ok(())
        } else {
            Err(pyo3::exceptions::PyOSError::new_err("Failed to send mouse event via SendInput"))
        }
    }

    #[pyfunction]
    pub fn send_keyboard_event(vk: u8, scan: u8, flags: u32) -> PyResult<()> {
        let vsc = if scan == 0 {
            unsafe { MapVirtualKeyA(vk as u32, 0) as u16 }
        } else {
            scan as u16
        };

        if try_send_driver_keyboard(vk, vsc as u8, flags) {
            return Ok(());
        }

        let mut dw_flags = flags | KEYEVENTF_SCANCODE;
        let is_extended = match vk {
            0x21..=0x28 | 0x2D | 0x2E | 0x5B..=0x5D | 0xA3 | 0xA5 => true,
            _ => false,
        };
        if is_extended {
            dw_flags |= KEYEVENTF_EXTENDEDKEY;
        }

        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.r#type = INPUT_KEYBOARD;
        input.Anonymous.ki = KEYBDINPUT {
            wVk: 0, // Must be 0 when KEYEVENTF_SCANCODE is specified
            wScan: vsc,
            dwFlags: dw_flags,
            time: unsafe { GetTickCount() },
            dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
        };

        let sent = unsafe {
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32)
        };
        if sent == 1 {
            Ok(())
        } else {
            Err(pyo3::exceptions::PyOSError::new_err("Failed to send keyboard event via SendInput"))
        }
    }

    #[pyfunction]
    pub fn vk_key_scan_a(c: u8) -> PyResult<i16> {
        let val = unsafe { VkKeyScanA(c) };
        Ok(val)
    }

    #[pyfunction]
    pub fn mouse_is_swapped() -> PyResult<bool> {
        let val = unsafe { GetSystemMetrics(SM_SWAPBUTTON) };
        Ok(val != 0)
    }

    #[pyfunction]
    pub fn move_rel(dx: i32, dy: i32) -> PyResult<()> {
        if try_send_driver_mouse(0, 0, dx, dy) {
            return Ok(());
        }

        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx,
            dy,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE,
            time: unsafe { GetTickCount() },
            dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
        };

        let sent = unsafe {
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32)
        };
        if sent == 1 {
            Ok(())
        } else {
            Err(pyo3::exceptions::PyOSError::new_err("Failed to send relative mouse move via SendInput"))
        }
    }

    #[pyfunction]
    pub fn send_inputs(py: Python<'_>, events: Vec<Bound<'_, PyTuple>>) -> PyResult<u32> {
        let mut win_inputs = Vec::with_capacity(events.len());
        for event in events {
            let event_type: u32 = event.get_item(0)?.extract()?;
            if event_type == 0 {
                let dx: i32 = event.get_item(1)?.extract()?;
                let dy: i32 = event.get_item(2)?.extract()?;
                let mouse_data: u32 = event.get_item(3)?.extract()?;
                let flags: u32 = event.get_item(4)?.extract()?;
                
                let mut button_flags = 0u16;
                if (flags & 0x0002) != 0 { button_flags |= 0x0001; }
                if (flags & 0x0004) != 0 { button_flags |= 0x0002; }
                if (flags & 0x0008) != 0 { button_flags |= 0x0004; }
                if (flags & 0x0010) != 0 { button_flags |= 0x0008; }
                if (flags & 0x0020) != 0 { button_flags |= 0x0010; }
                if (flags & 0x0040) != 0 { button_flags |= 0x0020; }
                if (flags & 0x0800) != 0 { button_flags |= 0x0400; }
                
                let mouse_flags = if (flags & 0x8000) != 0 { 1 } else { 0 };
                
                if try_send_driver_mouse(mouse_flags, button_flags, dx, dy) {
                    continue;
                }

                let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
                let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
                if width == 0 || height == 0 {
                    return Err(pyo3::exceptions::PyOSError::new_err("Failed to get screen metrics"));
                }
                
                let converted_x = (dx * 65535) / (width - 1);
                let converted_y = (dy * 65535) / (height - 1);

                let mut input = unsafe { std::mem::zeroed::<INPUT>() };
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: converted_x,
                    dy: converted_y,
                    mouseData: mouse_data,
                    dwFlags: flags | MOUSEEVENTF_ABSOLUTE,
                    time: unsafe { GetTickCount() },
                    dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
                };
                win_inputs.push(input);
            } else if event_type == 1 {
                let vk: u8 = event.get_item(1)?.extract()?;
                let scan: u8 = event.get_item(2)?.extract()?;
                let flags: u32 = event.get_item(3)?.extract()?;
                
                let vsc = if scan == 0 {
                    unsafe { MapVirtualKeyA(vk as u32, 0) as u16 }
                } else {
                    scan as u16
                };

                if try_send_driver_keyboard(vk, vsc as u8, flags) {
                    continue;
                }

                let mut dw_flags = flags | KEYEVENTF_SCANCODE;
                let is_extended = match vk {
                    0x21..=0x28 | 0x2D | 0x2E | 0x5B..=0x5D | 0xA3 | 0xA5 => true,
                    _ => false,
                };
                if is_extended {
                    dw_flags |= KEYEVENTF_EXTENDEDKEY;
                }

                let mut input = unsafe { std::mem::zeroed::<INPUT>() };
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: 0,
                    wScan: vsc,
                    dwFlags: dw_flags,
                    time: unsafe { GetTickCount() },
                    dwExtraInfo: unsafe { GetMessageExtraInfo() as usize },
                };
                win_inputs.push(input);
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err(format!("Invalid event type: {}", event_type)));
            }
        }
        
        let sent = unsafe {
            SendInput(
                win_inputs.len() as u32,
                win_inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        Ok(sent as u32)
    }

    #[pyfunction]
    pub fn move_to_smooth(x: i32, y: i32, duration: f64, steps: u32) -> PyResult<()> {
        let mut point = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut point) };
        if ok == 0 {
            return Err(pyo3::exceptions::PyOSError::new_err("Failed to get current cursor position"));
        }
        let x1 = point.x;
        let y1 = point.y;
        
        if duration <= 0.0 || steps <= 1 {
            set_cursor_pos(x, y)?;
            return Ok(());
        }
        
        let step_dur = std::time::Duration::from_secs_f64(duration / (steps as f64));
        for i in 1..=steps {
            let t = (i as f64) / (steps as f64);
            let cx = x1 + ((x as f64 - x1 as f64) * t).round() as i32;
            let cy = y1 + ((y as f64 - y1 as f64) * t).round() as i32;
            set_cursor_pos(cx, cy)?;
            if i < steps {
                std::thread::sleep(step_dur);
            }
        }
        Ok(())
    }

    #[pyfunction]
    pub fn capture_screen_gdi(py: Python<'_>, region: Option<(i32, i32, i32, i32)>) -> PyResult<PyObject> {
        unsafe {
            let (left, top, width, height) = match region {
                Some((r_left, r_top, r_width, r_height)) => (r_left, r_top, r_width, r_height),
                None => {
                    let w = GetSystemMetrics(0); // SM_CXSCREEN
                    let h = GetSystemMetrics(1); // SM_CYSCREEN
                    (0, 0, w, h)
                }
            };
            
            if width <= 0 || height <= 0 {
                return Err(pyo3::exceptions::PyValueError::new_err("Invalid capture width or height"));
            }
            
            let hwnd_desktop = 0 as _;
            let hdc_screen = GetDC(hwnd_desktop);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
            
            let old_obj = SelectObject(hdc_mem, hbmp);
            
            let ok = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, left, top, SRCCOPY);
            if ok == 0 {
                DeleteObject(hbmp);
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd_desktop, hdc_screen);
                return Err(pyo3::exceptions::PyOSError::new_err("BitBlt failed"));
            }
            
            let mut bmi = std::mem::zeroed::<BITMAPINFO>();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB as _;
            
            let buffer_size = (width * height * 4) as usize;
            let mut buffer = vec![0u8; buffer_size];
            
            let lines = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                height as u32,
                buffer.as_mut_ptr() as _,
                &mut bmi,
                DIB_RGB_COLORS,
            );
            
            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbmp);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd_desktop, hdc_screen);
            
            if lines == 0 {
                return Err(pyo3::exceptions::PyOSError::new_err("GetDIBits failed"));
            }
            
            let py_bytes = pyo3::types::PyBytes::new(py, &buffer);
            Ok(py_bytes.into())
        }
    }

    #[pyfunction]
    pub fn locate_on_screen_rust(
        py: Python<'_>,
        needle_bytes: &[u8],
        needle_w: usize,
        needle_h: usize,
        confidence: f32,
        region: Option<(i32, i32, i32, i32)>
    ) -> PyResult<Option<(i32, i32, i32, i32)>> {
        unsafe {
            let (left, top, width, height) = match region {
                Some((r_left, r_top, r_width, r_height)) => (r_left, r_top, r_width, r_height),
                None => {
                    let w = GetSystemMetrics(0);
                    let h = GetSystemMetrics(1);
                    (0, 0, w, h)
                }
            };
            
            if width <= 0 || height <= 0 || (width as usize) < needle_w || (height as usize) < needle_h {
                return Ok(None);
            }
            
            let hwnd_desktop = 0 as _;
            let hdc_screen = GetDC(hwnd_desktop);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
            let old_obj = SelectObject(hdc_mem, hbmp);
            
            let ok = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, left, top, SRCCOPY);
            if ok == 0 {
                DeleteObject(hbmp);
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd_desktop, hdc_screen);
                return Err(pyo3::exceptions::PyOSError::new_err("BitBlt failed"));
            }
            
            let mut bmi = std::mem::zeroed::<BITMAPINFO>();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB as _;
            
            let buffer_size = (width * height * 4) as usize;
            let mut screen_bgra = vec![0u8; buffer_size];
            
            let lines = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                height as u32,
                screen_bgra.as_mut_ptr() as _,
                &mut bmi,
                DIB_RGB_COLORS,
            );
            
            SelectObject(hdc_mem, old_obj);
            DeleteObject(hbmp);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd_desktop, hdc_screen);
            
            if lines == 0 {
                return Err(pyo3::exceptions::PyOSError::new_err("GetDIBits failed"));
            }
            
            let scr_w = width as usize;
            let scr_h = height as usize;
            let mut screen_gray = vec![0u8; scr_w * scr_h];
            for i in 0..scr_w * scr_h {
                let b = screen_bgra[i * 4];
                let g = screen_bgra[i * 4 + 1];
                let r = screen_bgra[i * 4 + 2];
                screen_gray[i] = ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8) as u8;
            }
            
            let result = match_template_hierarchical(
                &screen_gray, scr_w, scr_h,
                needle_bytes, needle_w, needle_h,
                confidence
            );
            
            if let Some((mx, my, _conf)) = result {
                Ok(Some((left + mx as i32, top + my as i32, needle_w as i32, needle_h as i32)))
            } else {
                Ok(None)
            }
        }
    }

    fn downscale_kb(src: &[u8], src_w: usize, src_h: usize, scale: usize) -> (Vec<u8>, usize, usize) {
        let dst_w = src_w / scale;
        let dst_h = src_h / scale;
        let mut dst = vec![0u8; dst_w * dst_h];
        for y in 0..dst_h {
            let src_y = y * scale;
            let src_offset = src_y * src_w;
            let dst_offset = y * dst_w;
            for x in 0..dst_w {
                dst[dst_offset + x] = src[src_offset + x * scale];
            }
        }
        (dst, dst_w, dst_h)
    }

    fn match_template_sad_rust(
        haystack: &[u8], h_w: usize, h_h: usize,
        needle: &[u8], n_w: usize, n_h: usize,
        min_confidence: f32,
        search_rect: Option<(usize, usize, usize, usize)>
    ) -> Vec<(usize, usize, f32)> {
        let mut results = Vec::new();
        
        let (start_x, start_y, limit_w, limit_h) = match search_rect {
            Some((sx, sy, sw, sh)) => (sx, sy, sw, sh),
            None => (0, 0, h_w, h_h)
        };
        
        if limit_w < n_w || limit_h < n_h { return results; }
        
        let max_diff = (n_w * n_h * 255) as f32;
        let max_sad_allowed = ((1.0 - min_confidence) * max_diff) as u32;
        
        let end_x = start_x + (limit_w - n_w);
        let end_y = start_y + (limit_h - n_h);
        
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let mut sad = 0u32;
                let mut broken = false;
                for py in 0..n_h {
                    let h_offset = (y + py) * h_w + x;
                    let n_offset = py * n_w;
                    for px in 0..n_w {
                        let h_val = haystack[h_offset + px];
                        let n_val = needle[n_offset + px];
                        sad += h_val.abs_diff(n_val) as u32;
                    }
                    if sad > max_sad_allowed {
                        broken = true;
                        break;
                    }
                }
                if !broken {
                    let conf = 1.0 - (sad as f32 / max_diff);
                    results.push((x, y, conf));
                }
            }
        }
        results
    }

    fn match_template_hierarchical(
        haystack: &[u8], h_w: usize, h_h: usize,
        needle: &[u8], n_w: usize, n_h: usize,
        min_confidence: f32
    ) -> Option<(usize, usize, f32)> {
        if n_w < 32 || n_h < 32 {
            let matches = match_template_sad_rust(haystack, h_w, h_h, needle, n_w, n_h, min_confidence, None);
            return matches.into_iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        }
        
        let scale = 4;
        let (h_down, hd_w, hd_h) = downscale_kb(haystack, h_w, h_h, scale);
        let (n_down, nd_w, nd_h) = downscale_kb(needle, n_w, n_h, scale);
        
        let coarse_threshold = (min_confidence - 0.08).max(0.5);
        let coarse_matches = match_template_sad_rust(&h_down, hd_w, hd_h, &n_down, nd_w, nd_h, coarse_threshold, None);
        
        let mut candidates = coarse_matches;
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        
        let mut best_match: Option<(usize, usize, f32)> = None;
        
        for (cx, cy, _) in candidates.into_iter().take(40) {
            let ox = cx * scale;
            let oy = cy * scale;
            
            let pad = 8;
            let sx1 = ox.saturating_sub(pad);
            let sy1 = oy.saturating_sub(pad);
            let sx2 = (ox + n_w + pad).min(h_w);
            let sy2 = (oy + n_h + pad).min(h_h);
            
            let local_w = sx2 - sx1;
            let local_h = sy2 - sy1;
            if local_w < n_w || local_h < n_h { continue; }
            
            let local_matches = match_template_sad_rust(
                haystack, h_w, h_h,
                needle, n_w, n_h,
                min_confidence,
                Some((sx1, sy1, local_w, local_h))
            );
            
            if let Some(m) = local_matches.into_iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap()) {
                if best_match.is_none() || m.2 > best_match.unwrap().2 {
                    best_match = Some(m);
                }
            }
        }
        
        best_match
    }

    unsafe extern "system" fn low_level_mouse_proc(code: i32, wparam: usize, lparam: isize) -> isize {
        if code >= 0 {
            let ms = &*(lparam as *const MSLLHOOKSTRUCT);
            let x = ms.pt.x;
            let y = ms.pt.y;
            
            let width = GetSystemMetrics(0);
            let height = GetSystemMetrics(1);
            
            if (x == 0 && y == 0)
                || (x == 0 && y == height - 1)
                || (x == width - 1 && y == 0)
                || (x == width - 1 && y == height - 1)
            {
                super::FAILSAFE_TRIGGERED.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        CallNextHookEx(0, code, wparam, lparam)
    }

    #[pyfunction]
    pub fn start_failsafe_hook() -> PyResult<()> {
        super::HOOK_ONCE.call_once(|| {
            std::thread::spawn(|| {
                unsafe {
                    let hook = SetWindowsHookExA(
                        WH_MOUSE_LL,
                        Some(low_level_mouse_proc),
                        0 as _,
                        0,
                    );
                    if hook == 0 {
                        return;
                    }
                    
                    let mut msg = std::mem::zeroed::<MSG>();
                    while GetMessageA(&mut msg, 0 as _, 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageA(&msg);
                    }
                    
                    UnhookWindowsHookEx(hook);
                }
            });
        });
        Ok(())
    }

    #[pyfunction]
    pub fn check_failsafe_triggered() -> PyResult<bool> {
        Ok(super::FAILSAFE_TRIGGERED.load(std::sync::atomic::Ordering::SeqCst))
    }

    #[pyfunction]
    pub fn reset_failsafe_triggered() -> PyResult<()> {
        super::FAILSAFE_TRIGGERED.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
mod dummy {
    use pyo3::prelude::*;
    use pyo3::types::PyTuple;

    #[pyfunction]
    pub fn set_process_dpi_aware() -> PyResult<bool> {
        Ok(false)
    }

    #[pyfunction]
    pub fn time_begin_period(_period: u32) -> PyResult<u32> {
        Ok(0)
    }

    #[pyfunction]
    pub fn time_end_period(_period: u32) -> PyResult<u32> {
        Ok(0)
    }

    #[pyfunction]
    pub fn get_cursor_pos() -> PyResult<(i32, i32)> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn get_system_metrics(_index: i32) -> PyResult<i32> {
        Ok(0)
    }

    #[pyfunction]
    pub fn set_cursor_pos(_x: i32, _y: i32) -> PyResult<()> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn send_mouse_event(_ev: u32, _x: i32, _y: i32, _data: i32) -> PyResult<()> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn send_keyboard_event(_vk: u8, _scan: u8, _flags: u32) -> PyResult<()> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn vk_key_scan_a(_c: u8) -> PyResult<i16> {
        Ok(-1)
    }

    #[pyfunction]
    pub fn mouse_is_swapped() -> PyResult<bool> {
        Ok(false)
    }

    #[pyfunction]
    pub fn move_rel(_dx: i32, _dy: i32) -> PyResult<()> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn send_inputs(_events: Vec<Bound<'_, PyTuple>>) -> PyResult<u32> {
        Ok(0)
    }

    #[pyfunction]
    pub fn move_to_smooth(_x: i32, _y: i32, _duration: f64, _steps: u32) -> PyResult<()> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn capture_screen_gdi(py: Python<'_>, _region: Option<(i32, i32, i32, i32)>) -> PyResult<PyObject> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn start_failsafe_hook() -> PyResult<()> {
        Ok(())
    }

    #[pyfunction]
    pub fn check_failsafe_triggered() -> PyResult<bool> {
        Ok(false)
    }

    #[pyfunction]
    pub fn reset_failsafe_triggered() -> PyResult<()> {
        Ok(())
    }

    #[pyfunction]
    pub fn locate_on_screen_rust(
        _py: Python<'_>,
        _needle_bytes: &[u8],
        _needle_w: usize,
        _needle_h: usize,
        _confidence: f32,
        _region: Option<(i32, i32, i32, i32)>
    ) -> PyResult<Option<(i32, i32, i32, i32)>> {
        Err(pyo3::exceptions::PyOSError::new_err("Not implemented on this platform"))
    }

    #[pyfunction]
    pub fn set_use_driver(_use_driver: bool) -> PyResult<()> {
        Ok(())
    }
}

#[pymodule]
fn _rust_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[cfg(target_os = "windows")]
    {
        m.add_function(wrap_pyfunction!(win::set_process_dpi_aware, m)?)?;
        m.add_function(wrap_pyfunction!(win::get_cursor_pos, m)?)?;
        m.add_function(wrap_pyfunction!(win::get_system_metrics, m)?)?;
        m.add_function(wrap_pyfunction!(win::set_cursor_pos, m)?)?;
        m.add_function(wrap_pyfunction!(win::send_mouse_event, m)?)?;
        m.add_function(wrap_pyfunction!(win::send_keyboard_event, m)?)?;
        m.add_function(wrap_pyfunction!(win::vk_key_scan_a, m)?)?;
        m.add_function(wrap_pyfunction!(win::mouse_is_swapped, m)?)?;
        m.add_function(wrap_pyfunction!(win::move_rel, m)?)?;
        m.add_function(wrap_pyfunction!(win::time_begin_period, m)?)?;
        m.add_function(wrap_pyfunction!(win::time_end_period, m)?)?;
        m.add_function(wrap_pyfunction!(win::send_inputs, m)?)?;
        m.add_function(wrap_pyfunction!(win::move_to_smooth, m)?)?;
        m.add_function(wrap_pyfunction!(win::capture_screen_gdi, m)?)?;
        m.add_function(wrap_pyfunction!(win::start_failsafe_hook, m)?)?;
        m.add_function(wrap_pyfunction!(win::check_failsafe_triggered, m)?)?;
        m.add_function(wrap_pyfunction!(win::reset_failsafe_triggered, m)?)?;
        m.add_function(wrap_pyfunction!(win::locate_on_screen_rust, m)?)?;
        m.add_function(wrap_pyfunction!(win::set_use_driver, m)?)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        m.add_function(wrap_pyfunction!(dummy::set_process_dpi_aware, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::get_cursor_pos, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::get_system_metrics, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::set_cursor_pos, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::send_mouse_event, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::send_keyboard_event, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::vk_key_scan_a, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::mouse_is_swapped, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::move_rel, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::time_begin_period, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::time_end_period, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::send_inputs, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::move_to_smooth, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::capture_screen_gdi, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::start_failsafe_hook, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::check_failsafe_triggered, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::reset_failsafe_triggered, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::locate_on_screen_rust, m)?)?;
        m.add_function(wrap_pyfunction!(dummy::set_use_driver, m)?)?;
    }
    Ok(())
}
