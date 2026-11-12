// Stowaway app shell: the window, the menu-bar popover and a View > Appearance menu. All data goes through
// app/ui/src/api.ts (the agent socket after integration); this process only hosts the UI.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod icon;

use std::path::Path;
use std::process::Command;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_window_state::StateFlags;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WindowEvent, Wry};

const THEMES: [(&str, &str); 3] = [("system", "System"), ("light", "Light"), ("dark", "Dark")];

/// PNG of an app's real icon, read from its bundle. Only `.app` bundles; never sandbox containers.
#[tauri::command]
fn app_icon(path: String) -> Result<tauri::ipc::Response, String> {
    if !path.ends_with(".app") || !Path::new(&path).is_dir() {
        return Err("not an app bundle".into());
    }
    icon::png(&path, 56).map(tauri::ipc::Response::new).ok_or_else(|| "no icon".into())
}

/// Opens one of a fixed set of places, so a "fix" button in an error state does something real.
#[tauri::command]
fn open_target(target: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let journal = format!("{home}/Library/Application Support/Stowaway/journal.jsonl");
    let args: Vec<&str> = match target {
        "login-items" => vec!["x-apple.systempreferences:com.apple.LoginItems-Settings.extension"],
        "privacy" => vec!["x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders"],
        "disk-utility" => vec!["-b", "com.apple.DiskUtility"],
        "journal" if Path::new(&journal).exists() => vec!["-R", &journal],
        "journal" => return Err("There is no journal yet. It is created with the first move.".into()),
        _ => return Err(format!("unknown target {target}")),
    };
    let ok = Command::new("/usr/bin/open").args(&args).status().map_err(|e| e.to_string())?.success();
    ok.then_some(()).ok_or_else(|| "macOS couldn’t open it.".into())
}

/// Keeps the View > Appearance checkmarks in step with the saved setting.
#[tauri::command]
fn appearance_checked(app: AppHandle, theme: String) {
    if let Some(menu) = app.menu() {
        for (id, _) in THEMES {
            if let Some(MenuItemKind::Check(c)) = menu.get(&format!("appearance:{id}")) {
                let _ = c.set_checked(id == theme);
            }
        }
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    let items: Vec<CheckMenuItem<Wry>> = THEMES
        .iter()
        .map(|(id, label)| CheckMenuItem::with_id(app, format!("appearance:{id}"), *label, true, *id == "system", None::<&str>))
        .collect::<tauri::Result<_>>()?;
    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = items.iter().map(|i| i as _).collect();
    let appearance = Submenu::with_items(app, "Appearance", true, &refs)?;
    let view = menu.items()?.into_iter().find_map(|i| match i {
        MenuItemKind::Submenu(s) if s.text().ok().as_deref() == Some("View") => Some(s),
        _ => None,
    });
    match view {
        Some(v) => {
            v.prepend(&PredefinedMenuItem::separator(app)?)?;
            v.prepend(&appearance)?;
        }
        None => menu.append(&Submenu::with_items(app, "View", true, &[&appearance])?)?,
    }
    Ok(menu)
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        // Size and position only: closing hides the window, and a remembered "hidden" would launch with no window.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["popover"])
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![app_icon, open_target, appearance_checked])
        .setup(|app| {
            let handle = app.handle();
            app.set_menu(menu(handle)?)?;
            app.on_menu_event(|app, e| {
                let id = e.id().as_ref();
                if let Some(theme) = id.strip_prefix("appearance:") {
                    let _ = app.emit("appearance", theme);
                } else if id == "tray:open" {
                    show_main(app);
                } else if id == "tray:quit" {
                    app.exit(0);
                }
            });

            // Menu-bar item: left click toggles the popover under it, right click offers Open and Quit.
            let tray_menu = Menu::with_items(
                handle,
                &[
                    &MenuItem::with_id(handle, "tray:open", "Open Stowaway", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(handle)?,
                    &MenuItem::with_id(handle, "tray:quit", "Quit Stowaway", true, None::<&str>)?,
                ],
            )?;
            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .tooltip("Stowaway")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, ev| {
                    let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = ev
                    else {
                        return;
                    };
                    let Some(pop) = tray.app_handle().get_webview_window("popover") else { return };
                    if pop.is_visible().unwrap_or(false) {
                        let _ = pop.hide();
                        return;
                    }
                    let scale = pop.scale_factor().unwrap_or(2.0);
                    let (pos, size) = (rect.position.to_physical::<f64>(scale), rect.size.to_physical::<f64>(scale));
                    let x = pos.x + size.width / 2.0 - 170.0 * scale;
                    let _ = pop.set_position(PhysicalPosition::new(x, pos.y + size.height + 4.0 * scale));
                    let _ = pop.show();
                    let _ = pop.set_focus();
                })
                .build(app)?;

            if let Some(pop) = app.get_webview_window("popover") {
                let p = pop.clone();
                pop.on_window_event(move |e| {
                    if let WindowEvent::Focused(false) = e {
                        let _ = p.hide();
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Stowaway")
        .run(|app, event| match event {
            // Closing the window keeps Stowaway in the menu bar; clicking the Dock icon brings it back.
            RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. } if label == "main" => {
                api.prevent_close();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            RunEvent::Reopen { .. } => show_main(app),
            _ => {}
        });
}
