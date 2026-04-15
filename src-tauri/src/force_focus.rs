#[cfg(target_os = "linux")]
pub fn activate_by_title(title: &str) {
    if let Err(e) = try_activate(title) {
        eprintln!("[polishd] force_focus: {e}");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn activate_by_title(_title: &str) {}

#[cfg(target_os = "linux")]
fn try_activate(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ClientMessageEvent, ConnectionExt, EventMask,
    };
    use x11rb::rust_connection::RustConnection;

    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atom = |name: &[u8]| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(conn.intern_atom(false, name)?.reply()?.atom)
    };

    let net_client_list = atom(b"_NET_CLIENT_LIST")?;
    let net_wm_name = atom(b"_NET_WM_NAME")?;
    let utf8_string = atom(b"UTF8_STRING")?;
    let net_active_window = atom(b"_NET_ACTIVE_WINDOW")?;

    let client_list = conn
        .get_property(false, root, net_client_list, AtomEnum::WINDOW, 0, 4096)?
        .reply()?;
    let windows: Vec<u32> = client_list
        .value32()
        .ok_or("_NET_CLIENT_LIST missing")?
        .collect();

    let mut target: Option<u32> = None;
    for &wid in windows.iter().rev() {
        let name_reply = match conn
            .get_property(false, wid, net_wm_name, utf8_string, 0, 1024)
        {
            Ok(cookie) => match cookie.reply() {
                Ok(r) => r,
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        let name = String::from_utf8_lossy(&name_reply.value);
        if name == title {
            target = Some(wid);
            break;
        }
    }

    let target = target.ok_or("transform window not in _NET_CLIENT_LIST")?;

    let event = ClientMessageEvent::new(
        32,
        target,
        net_active_window,
        [2u32, 0, 0, 0, 0],
    );
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT,
        event,
    )?;
    conn.flush()?;

    Ok(())
}
