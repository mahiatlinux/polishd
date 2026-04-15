use arboard::Clipboard;

pub fn save() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

pub fn read() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

pub fn write(text: String) -> bool {
    match Clipboard::new() {
        Ok(mut ctx) => ctx.set_text(text).is_ok(),
        Err(_) => false,
    }
}
