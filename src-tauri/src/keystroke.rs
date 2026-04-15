use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::{thread, time::Duration};

#[cfg(target_os = "macos")]
const MODIFIER: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const MODIFIER: Key = Key::Control;

pub fn copy() -> Result<(), String> {
    thread::sleep(Duration::from_millis(80));
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let _ = enigo.key(Key::Shift, Direction::Release);
    thread::sleep(Duration::from_millis(20));
    enigo.key(MODIFIER, Direction::Press).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(Key::Unicode('c'), Direction::Click).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(MODIFIER, Direction::Release).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn paste() -> Result<(), String> {
    thread::sleep(Duration::from_millis(80));
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(MODIFIER, Direction::Press).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(MODIFIER, Direction::Release).map_err(|e| e.to_string())?;
    Ok(())
}
