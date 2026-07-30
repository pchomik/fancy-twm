use anyhow::{Context, Result};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

pub struct TrayController {
    /// Must be kept alive for the tray icon to remain visible.
    #[allow(dead_code)]
    icon: TrayIcon,
    menu_items: Vec<MenuItem>,
}

impl TrayController {
    pub fn new() -> Result<Self> {
        let (menu, menu_items) = TrayController::create_menu();
        let icon = TrayIconBuilder::new()
            .with_icon(TrayController::load_icon()?)
            .with_menu(Box::new(menu))
            .with_tooltip("Tilo")
            .build()
            .context("Failed to start tray icon")?;
        Ok(Self { icon, menu_items })
    }

    fn create_menu() -> (Menu, Vec<MenuItem>) {
        let menu = Menu::new();
        let menu_items = vec![MenuItem::new("Quit", true, None)];
        for item in &menu_items {
            let _ = menu.append(item);
        }
        (menu, menu_items)
    }

    fn load_icon() -> Result<Icon> {
        let icon_data = include_bytes!("../assets/app.ico");
        let image = image::load_from_memory(icon_data)?.to_rgba8();
        let (width, height) = image.dimensions();
        Ok(Icon::from_rgba(image.into_raw(), width, height)?)
    }

    pub fn read(&self) {
        if let Ok(event) = MenuEvent::receiver().try_recv()
            && let Some(menu_item) = self.menu_items.iter().find(|&item| item.id() == event.id())
                && menu_item.text() == "Quit" {
                    std::process::exit(0);
                }
    }
}
