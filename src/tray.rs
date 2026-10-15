//! System tray icon, its menu, and the commands it raises.
//!
//! The tray is optional. When registration fails the application stays fully
//! usable from its window: start-hidden and close-to-tray are suppressed so a
//! missing tray can never strand the window out of reach.
//!
//! Platform note: on Windows and macOS a tray icon must be created on a thread
//! that is running an event loop, and on macOS that thread must be the main
//! one. Registration therefore happens inside the application constructor,
//! which eframe runs on the main thread once its event loop is up, rather than
//! from a worker.
//!
//! Menu events are delivered through the library's handler hook rather than by
//! polling its channel, because a command chosen while the window is hidden
//! must still reach the application: the handler runs on the event's own
//! thread and can wake the UI, whereas a frame that never runs would never
//! drain a queue.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::IconData;

/// A command raised by the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Show and focus the window on the translation view.
    Open,
    /// Show and focus the window on the settings view.
    Settings,
    /// Exit the application, cancelling any active request.
    Quit,
}

/// Menu item identifier for the open command.
const OPEN_ID: &str = "yalt-open";
/// Menu item identifier for the settings command.
const SETTINGS_ID: &str = "yalt-settings";
/// Menu item identifier for the quit command.
const QUIT_ID: &str = "yalt-quit";

/// Map a menu item identifier to a command.
fn command_for(id: &str) -> Option<Command> {
    match id {
        OPEN_ID => Some(Command::Open),
        SETTINGS_ID => Some(Command::Settings),
        QUIT_ID => Some(Command::Quit),
        _ => None,
    }
}

/// The application icon, decoded from an embedded PNG.
#[derive(Clone)]
pub struct AppIcon {
    /// Pixels as separate/unmultiplied RGBA.
    pub rgba: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

impl AppIcon {
    /// Build the icon from encoded PNG bytes.
    pub fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|error| format!("icon is not a readable PNG: {error}"))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        Ok(Self {
            rgba: image.into_raw(),
            width,
            height,
        })
    }

    /// The window/taskbar form egui expects.
    pub fn to_icon_data(&self) -> IconData {
        IconData {
            rgba: self.rgba.clone(),
            width: self.width,
            height: self.height,
        }
    }

    /// A scaled copy for the tray, which is drawn small.
    pub fn scaled(&self, size: u32) -> Self {
        let source = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
            .expect("the icon's pixels match its dimensions");
        let target =
            image::imageops::resize(&source, size, size, image::imageops::FilterType::Triangle);
        Self {
            rgba: target.into_raw(),
            width: size,
            height: size,
        }
    }
}

/// The embedded application icon.
///
/// Embedding keeps a packaged build self-contained, so the icon cannot be lost
/// by moving the executable away from its assets.
pub const ICON_PNG: &[u8] = include_bytes!("../build/appicon.png");

/// Load the embedded application icon.
pub fn load_icon() -> Option<AppIcon> {
    match AppIcon::from_png(ICON_PNG) {
        Ok(icon) => Some(icon),
        Err(error) => {
            eprintln!("yalt: {error}");
            None
        }
    }
}

/// Whether a tray backend is available on this platform.
///
/// Linux has no backend compiled in: the two available ones need either GTK
/// and libappindicator or a session D-Bus service, and neither can be assumed
/// on a machine that has to run the application regardless. Window-based
/// behaviour is unaffected.
pub const fn supported() -> bool {
    cfg!(any(target_os = "windows", target_os = "macos"))
}

/// A handle used to wake the UI from the tray's event thread.
#[derive(Clone)]
pub struct Waker {
    /// Called after a command is queued.
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl Waker {
    /// Wrap a repaint request.
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self { wake }
    }

    /// Wake the UI thread.
    pub fn wake(&self) {
        (self.wake)();
    }
}

/// The registered tray icon, kept alive for as long as the tray is enabled.
pub struct Tray {
    /// Dropping this removes the icon. Absent when registration failed or no
    /// backend exists.
    backend: Option<tray_icon::TrayIcon>,
    /// Whether registration succeeded.
    registered: bool,
}

impl Tray {
    /// Register a tray icon when one is enabled and available.
    ///
    /// Never fails: a tray that cannot start is reported through
    /// [`Tray::is_registered`] so the caller can keep the window reachable.
    pub fn new(
        enabled: bool,
        icon: Option<AppIcon>,
        commands: Sender<Command>,
        waker: Waker,
    ) -> Self {
        let mut tray = Self {
            backend: None,
            registered: false,
        };
        if enabled && supported() {
            tray.register(icon, commands, waker);
        }
        tray
    }

    /// Attempt registration, recording success or failure.
    fn register(&mut self, icon: Option<AppIcon>, commands: Sender<Command>, waker: Waker) {
        match build_backend(icon) {
            Ok(backend) => {
                install_handler(commands, waker);
                self.backend = Some(backend);
                self.registered = true;
                eprintln!("yalt: tray registered");
            }
            Err(error) => {
                eprintln!("yalt: tray unavailable, using window controls: {error}");
            }
        }
    }

    /// Whether the tray is registered and can be relied on.
    pub fn is_registered(&self) -> bool {
        self.registered
    }
}

/// Create the platform tray icon and its menu.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn build_backend(icon: Option<AppIcon>) -> Result<tray_icon::TrayIcon, String> {
    use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tray_icon::{TrayIcon, TrayIconBuilder};

    let menu = Menu::new();
    let open = MenuItem::with_id(OPEN_ID, "Open Translator", true, None);
    let settings = MenuItem::with_id(SETTINGS_ID, "Settings", true, None);
    let quit = MenuItem::with_id(QUIT_ID, "Quit", true, None);
    menu.append_items(&[&open, &settings, &PredefinedMenuItem::separator(), &quit])
        .map_err(|error| format!("could not build the tray menu: {error}"))?;

    let mut builder = TrayIconBuilder::new()
        .with_tooltip("yalt")
        .with_menu(Box::new(menu));
    if let Some(icon) = icon {
        let small = icon.scaled(TRAY_ICON_SIZE);
        let image = tray_icon::Icon::from_rgba(small.rgba, small.width, small.height)
            .map_err(|error| format!("tray icon is invalid: {error}"))?;
        builder = builder.with_icon(image);
    }

    let icon: TrayIcon = builder
        .build()
        .map_err(|error| format!("could not register the tray icon: {error}"))?;
    Ok(icon)
}

/// No backend is compiled in on this platform.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn build_backend(_icon: Option<AppIcon>) -> Result<tray_icon::TrayIcon, String> {
    Err("no tray backend is available on this platform".to_owned())
}

/// Forward menu events to a channel, waking the UI for each one.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn install_handler(commands: Sender<Command>, waker: Waker) {
    tray_icon::menu::MenuEvent::set_event_handler(Some(
        move |event: tray_icon::menu::MenuEvent| {
            if let Some(command) = command_for(event.id.as_ref()) {
                let _ = commands.send(command);
                waker.wake();
            }
        },
    ));
}

/// No menu exists without a backend.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn install_handler(_commands: Sender<Command>, _waker: Waker) {}

/// Tray icons are drawn at a small size; 32 covers the common scales.
const TRAY_ICON_SIZE: u32 = 32;

/// A command channel for tray events.
pub fn command_channel() -> (Sender<Command>, Receiver<Command>) {
    channel()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_icon_decodes() {
        let icon = load_icon().expect("the embedded icon must decode");
        assert_eq!(icon.width, 256);
        assert_eq!(icon.height, 256);
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }

    #[test]
    fn the_window_icon_is_usable_by_egui() {
        let icon = load_icon().expect("the embedded icon must decode");
        let data = icon.to_icon_data();
        assert_eq!(data.width, 256);
        assert_eq!(data.rgba.len(), (data.width * data.height * 4) as usize);
    }

    #[test]
    fn the_tray_icon_is_scaled_to_the_expected_size() {
        let icon = load_icon().expect("the embedded icon must decode");
        let small = icon.scaled(TRAY_ICON_SIZE);
        assert_eq!(small.width, TRAY_ICON_SIZE);
        assert_eq!(small.height, TRAY_ICON_SIZE);
        assert_eq!(
            small.rgba.len(),
            (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize
        );
    }

    #[test]
    fn menu_identifiers_map_to_their_commands() {
        assert_eq!(command_for(OPEN_ID), Some(Command::Open));
        assert_eq!(command_for(SETTINGS_ID), Some(Command::Settings));
        assert_eq!(command_for(QUIT_ID), Some(Command::Quit));
        assert_eq!(command_for("something-else"), None);
    }

    #[test]
    fn a_tray_that_cannot_start_is_reported_as_unregistered() {
        // The window must stay reachable when no tray exists, so this is the
        // state that suppresses start-hidden and close-to-tray.
        let (commands, _receiver) = command_channel();
        let waker = Waker::new(Arc::new(|| {}));
        let tray = Tray::new(false, None, commands, waker);
        assert!(!tray.is_registered());
    }

    #[test]
    fn the_waker_is_invoked() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        let waker = Waker::new(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        waker.wake();
        waker.wake();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
