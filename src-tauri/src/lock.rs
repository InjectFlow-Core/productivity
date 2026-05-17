use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

/// Force the window to cover the entire screen by bypassing the window manager.
/// override_redirect makes the WM ignore the window completely, so panels,
/// taskbars, and WM decorations cannot overlap it.
pub fn setup_window(window_id: u32) -> Result<(), String> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let sw = screen.width_in_pixels as u32;
    let sh = screen.height_in_pixels as u32;

    // Unmap first — override_redirect must be set before the window is visible
    // to take full effect on all WMs.
    conn.unmap_window(window_id)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    conn.change_window_attributes(
        window_id,
        &ChangeWindowAttributesAux::new().override_redirect(1),
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;

    // Move to top-left and cover the full screen
    conn.configure_window(
        window_id,
        &ConfigureWindowAux::new()
            .x(0)
            .y(0)
            .width(sw)
            .height(sh)
            .stack_mode(StackMode::ABOVE),
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;

    // Remap — window reappears covering the full screen, WM-ignored
    conn.map_window(window_id)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Shrink the window to a normal dashboard size and center it on screen.
/// Keeps override_redirect so the WM doesn't try to re-adopt the window;
/// the frontend renders its own close button instead.
pub fn normalize_window(xid: u32) -> Result<(), String> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let sw = screen.width_in_pixels as u32;
    let sh = screen.height_in_pixels as u32;

    let w = 960u32;
    let h = 700u32;
    let x = ((sw.saturating_sub(w)) / 2) as i32;
    let y = ((sh.saturating_sub(h)) / 2) as i32;

    conn.configure_window(
        xid,
        &ConfigureWindowAux::new()
            .x(x)
            .y(y)
            .width(w)
            .height(h)
            .stack_mode(StackMode::ABOVE),
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;

    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn grab_input(window_id: u32) -> Result<(), String> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;

    // With override_redirect the WM won't give us focus, so we must request it
    // explicitly. PARENT revert means focus falls back to the parent (root)
    // if this window is unmapped.
    conn.set_input_focus(
        InputFocus::PARENT,
        window_id,
        x11rb::CURRENT_TIME,
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;

    // Grab the keyboard so key events are routed only to our window even if the
    // pointer happens to be over another window.
    let grab = conn
        .grab_keyboard(
            false,
            window_id,
            x11rb::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;

    if grab.status != GrabStatus::SUCCESS {
        return Err(format!("grab_keyboard failed with status {:?}", grab.status));
    }

    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn release_input() -> Result<(), String> {
    let (conn, _) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    conn.ungrab_keyboard(x11rb::CURRENT_TIME)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}
