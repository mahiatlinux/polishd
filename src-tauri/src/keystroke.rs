use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::{thread, time::Duration};

#[cfg(target_os = "macos")]
const MODIFIER: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const MODIFIER: Key = Key::Control;

fn release_stuck_modifiers(enigo: &mut Enigo) {
    let _ = enigo.key(Key::Shift, Direction::Release);
    let _ = enigo.key(Key::Control, Direction::Release);
    let _ = enigo.key(Key::Alt, Direction::Release);
    #[cfg(target_os = "macos")]
    let _ = enigo.key(Key::Meta, Direction::Release);
}

pub fn copy() -> Result<(), String> {
    thread::sleep(Duration::from_millis(80));
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    release_stuck_modifiers(&mut enigo);
    thread::sleep(Duration::from_millis(40));
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
    release_stuck_modifiers(&mut enigo);
    thread::sleep(Duration::from_millis(40));
    enigo.key(MODIFIER, Direction::Press).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(20));
    enigo.key(MODIFIER, Direction::Release).map_err(|e| e.to_string())?;
    Ok(())
}
