// virtio-input: keyboard and mouse event translation
// Mouse: never captured. Tracks position over canvas, translates to VM coords.
// Keyboard: passthrough when node focused. No lock/capture by default.
// Keycodes emitted are X11 keysyms (RFB-compatible).

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    KeyDown { code: String, key: String },
    KeyUp   { code: String, key: String },
    MouseMove { x: f64, y: f64, node_w: f64, node_h: f64 },
    MouseDown { x: f64, y: f64, button: u8, node_w: f64, node_h: f64 },
    MouseUp   { x: f64, y: f64, button: u8, node_w: f64, node_h: f64 },
    Wheel     { delta_x: f64, delta_y: f64 },
}

#[derive(Debug, Clone)]
pub struct VmInputEvent {
    pub kind: VmInputKind,
}

#[derive(Debug, Clone)]
pub enum VmInputKind {
    KeyDown(u32),  // X11 keysym
    KeyUp(u32),    // X11 keysym
    MouseMove(i32, i32),
    MouseButton { x: i32, y: i32, button: u8, pressed: bool },
    MouseWheel  { dx: i32, dy: i32 },
}

pub struct InputDevice {
    pub fb_w: u32,
    pub fb_h: u32,
}

impl InputDevice {
    pub fn new(fb_w: u32, fb_h: u32) -> Self { Self { fb_w, fb_h } }

    pub fn translate(&self, event: InputEvent) -> Option<VmInputEvent> {
        match event {
            InputEvent::KeyDown { code, key } => {
                let ks = web_code_to_keysym(&code, &key)?;
                Some(VmInputEvent { kind: VmInputKind::KeyDown(ks) })
            }
            InputEvent::KeyUp { code, key } => {
                let ks = web_code_to_keysym(&code, &key)?;
                Some(VmInputEvent { kind: VmInputKind::KeyUp(ks) })
            }
            InputEvent::MouseMove { x, y, node_w, node_h } => {
                let vx = scale(x, node_w, self.fb_w);
                let vy = scale(y, node_h, self.fb_h);
                Some(VmInputEvent { kind: VmInputKind::MouseMove(vx, vy) })
            }
            InputEvent::MouseDown { x, y, button, node_w, node_h } => {
                let vx = scale(x, node_w, self.fb_w);
                let vy = scale(y, node_h, self.fb_h);
                Some(VmInputEvent { kind: VmInputKind::MouseButton { x: vx, y: vy, button, pressed: true } })
            }
            InputEvent::MouseUp { x, y, button, node_w, node_h } => {
                let vx = scale(x, node_w, self.fb_w);
                let vy = scale(y, node_h, self.fb_h);
                Some(VmInputEvent { kind: VmInputKind::MouseButton { x: vx, y: vy, button, pressed: false } })
            }
            InputEvent::Wheel { delta_x, delta_y } => {
                Some(VmInputEvent { kind: VmInputKind::MouseWheel {
                    dx: delta_x as i32,
                    dy: delta_y as i32,
                }})
            }
        }
    }
}

fn scale(pos: f64, node_dim: f64, fb_dim: u32) -> i32 {
    if node_dim <= 0.0 { return 0; }
    ((pos / node_dim) * fb_dim as f64).clamp(0.0, fb_dim as f64 - 1.0) as i32
}

// Web KeyboardEvent.code → X11 keysym
// X11 keysyms: https://cgit.freedesktop.org/xorg/proto/x11proto/tree/keysymdef.h
fn web_code_to_keysym(code: &str, key: &str) -> Option<u32> {
    Some(match code {
        // Control keys
        "Escape"         => 0xFF1B,
        "Backspace"      => 0xFF08,
        "Tab"            => 0xFF09,
        "Enter"          => 0xFF0D,
        "CapsLock"       => 0xFFE5,
        "ShiftLeft"      => 0xFFE1,
        "ShiftRight"     => 0xFFE2,
        "ControlLeft"    => 0xFFE3,
        "ControlRight"   => 0xFFE4,
        "AltLeft"        => 0xFFE9,
        "AltRight"       => 0xFFEA,
        "MetaLeft"       => 0xFFEB,
        "MetaRight"      => 0xFFEC,
        "Space"          => 0x0020,
        "Delete"         => 0xFFFF,
        "Insert"         => 0xFF63,
        "Home"           => 0xFF50,
        "End"            => 0xFF57,
        "PageUp"         => 0xFF55,
        "PageDown"       => 0xFF56,
        "ArrowLeft"      => 0xFF51,
        "ArrowUp"        => 0xFF52,
        "ArrowRight"     => 0xFF53,
        "ArrowDown"      => 0xFF54,
        "PrintScreen"    => 0xFF61,
        "ScrollLock"     => 0xFF14,
        "Pause"          => 0xFF13,
        "NumLock"        => 0xFF7F,
        // Function keys
        "F1"  => 0xFFBE, "F2"  => 0xFFBF, "F3"  => 0xFFC0, "F4"  => 0xFFC1,
        "F5"  => 0xFFC2, "F6"  => 0xFFC3, "F7"  => 0xFFC4, "F8"  => 0xFFC5,
        "F9"  => 0xFFC6, "F10" => 0xFFC7, "F11" => 0xFFC8, "F12" => 0xFFC9,
        // Numpad
        "Numpad0" => 0xFFB0, "Numpad1" => 0xFFB1, "Numpad2" => 0xFFB2,
        "Numpad3" => 0xFFB3, "Numpad4" => 0xFFB4, "Numpad5" => 0xFFB5,
        "Numpad6" => 0xFFB6, "Numpad7" => 0xFFB7, "Numpad8" => 0xFFB8,
        "Numpad9" => 0xFFB9,
        "NumpadDecimal"  => 0xFFAE, "NumpadEnter" => 0xFF8D,
        "NumpadAdd"      => 0xFFAB, "NumpadSubtract" => 0xFFAD,
        "NumpadMultiply" => 0xFFAA, "NumpadDivide"   => 0xFFAF,
        // For printable keys, use the key value if it's a single character
        _ => {
            if key.chars().count() == 1 {
                key.chars().next().unwrap() as u32
            } else {
                return None;
            }
        }
    })
}
