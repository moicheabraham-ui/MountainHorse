#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

const KEYRING_SERVICE: &str = "SftpDrive";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Connection {
    id: String,
    name: String,
    host: String,
    port: u16,
    user: String,
    remote_path: String,
    drive: String,
    use_key: bool,
    #[serde(default)]
    key_path: String,
    #[serde(default)]
    auto_mount: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    #[serde(flatten)]
    conn: Connection,
    mounted: bool,
}

fn err<E: ToString>(e: E) -> String {
    e.to_string()
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(err)?;
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join("connections.json"))
}

fn load(app: &AppHandle) -> Result<Vec<Connection>, String> {
    let p = store_path(app)?;
    if !p.exists() {
        return Ok(vec![]);
    }
    serde_json::from_str(&fs::read_to_string(p).map_err(err)?).map_err(err)
}

fn save(app: &AppHandle, list: &[Connection]) -> Result<(), String> {
    fs::write(store_path(app)?, serde_json::to_string_pretty(list).map_err(err)?).map_err(err)
}

// host/user end up inside an SSHFS-Win UNC path, so these characters would change its meaning
fn clean(s: &str) -> bool {
    !s.is_empty() && !s.contains(|c: char| c.is_whitespace() || "\\/!@".contains(c))
}

fn is_mounted(drive: &str) -> bool {
    Path::new(&format!("{}:\\", drive)).exists()
}

fn run_cmd(program: &str, args: &[String]) -> Result<(), String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().map_err(err)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&[out.stderr, out.stdout].concat()).trim().to_string())
    }
}

fn ssh_config_path() -> Result<PathBuf, String> {
    Ok(PathBuf::from(std::env::var("USERPROFILE").map_err(err)?).join(".ssh").join("config"))
}

// SSHFS-Win only reads %USERPROFILE%\.ssh\id_rsa by default; a custom key needs a Host alias in ssh config.
// We keep one marked block per connection so it can be replaced or removed cleanly.
fn set_ssh_block(id: &str, block: Option<String>) -> Result<(), String> {
    let path = ssh_config_path()?;
    let old = fs::read_to_string(&path).unwrap_or_default();
    let start = format!("# >>> SftpDrive {id}\n");
    let end = format!("# <<< SftpDrive {id}\n");
    let mut text = old.clone();
    if let (Some(a), Some(b)) = (text.find(&start), text.find(&end)) {
        if b > a {
            text.replace_range(a..b + end.len(), "");
        }
    }
    if let Some(block) = block {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&start);
        text.push_str(&block);
        text.push_str(&end);
    }
    if text != old {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(err)?;
        }
        fs::write(&path, text).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn list_connections(app: AppHandle) -> Result<Vec<View>, String> {
    Ok(load(&app)?
        .into_iter()
        .map(|conn| {
            let mounted = is_mounted(&conn.drive);
            View { conn, mounted }
        })
        .collect())
}

#[tauri::command]
fn save_connection(app: AppHandle, mut conn: Connection, password: Option<String>) -> Result<(), String> {
    conn.name = conn.name.trim().to_string();
    conn.drive = conn.drive.trim().trim_end_matches(':').to_uppercase();
    if conn.name.is_empty() || !clean(&conn.host) || !clean(&conn.user) || conn.port == 0 {
        return Err("Revisa nombre, servidor, usuario y puerto (sin espacios ni \\ / ! @).".into());
    }
    conn.key_path = conn.key_path.trim().to_string();
    if conn.key_path.contains(|c: char| c == '"' || c == '\n' || c == '\r') {
        return Err("La ruta de la clave no es válida.".into());
    }
    if conn.drive.len() != 1 || !conn.drive.chars().all(|c| c.is_ascii_uppercase()) {
        return Err("La unidad debe ser una letra (A-Z).".into());
    }
    if conn.id.is_empty() {
        conn.id = SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_millis().to_string();
    }
    if let Some(pw) = password.filter(|p| !p.is_empty()) {
        keyring::Entry::new(KEYRING_SERVICE, &conn.id).map_err(err)?.set_password(&pw).map_err(err)?;
    }
    let mut list = load(&app)?;
    match list.iter_mut().find(|c| c.id == conn.id) {
        Some(existing) => *existing = conn,
        None => list.push(conn),
    }
    save(&app, &list)?;
    refresh_tray(&app);
    Ok(())
}

#[tauri::command]
fn delete_connection(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load(&app)?;
    list.retain(|c| c.id != id);
    let _ = set_ssh_block(&id, None);
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &id) {
        let _ = entry.delete_credential();
    }
    save(&app, &list)?;
    refresh_tray(&app);
    Ok(())
}

fn mount_conn(app: &AppHandle, id: &str) -> Result<(), String> {
    let c = load(app)?.into_iter().find(|c| c.id == id).ok_or("Conexión no encontrada")?;
    if is_mounted(&c.drive) {
        return Err(format!("La unidad {}: ya está en uso.", c.drive));
    }
    // \\sshfs.r = path from the server root, \\sshfs.kr = same with key auth
    let prefix = if c.use_key { "sshfs.kr" } else { "sshfs.r" };
    let mut host = c.host.clone();
    if c.use_key && !c.key_path.is_empty() {
        let key = c.key_path.replace('\\', "/");
        set_ssh_block(
            &c.id,
            Some(format!("Host sftpdrive-{id}\n  HostName {}\n  Port {}\n  IdentityFile \"{key}\"\n", c.host, c.port, id = c.id)),
        )?;
        host = format!("sftpdrive-{}", c.id);
    } else {
        let _ = set_ssh_block(&c.id, None);
    }
    let path = c.remote_path.trim().replace('/', "\\");
    let path = if path.is_empty() || path.starts_with('\\') { path } else { format!("\\{}", path) };
    let unc = format!("\\\\{}\\{}@{}!{}{}", prefix, c.user, host, c.port, path);

    let mut args = vec!["use".to_string(), format!("{}:", c.drive), unc];
    if !c.use_key {
        let pw = keyring::Entry::new(KEYRING_SERVICE, &c.id)
            .and_then(|e| e.get_password())
            .map_err(|_| "Falta la contraseña: edita la conexión y guárdala.".to_string())?;
        args.push(pw);
    }
    args.push(format!("/user:{}", c.user));
    args.push("/persistent:no".into());
    run_cmd("net", &args)
}

fn unmount_conn(app: &AppHandle, id: &str) -> Result<(), String> {
    let c = load(app)?.into_iter().find(|c| c.id == id).ok_or("Conexión no encontrada")?;
    run_cmd("net", &["use".into(), format!("{}:", c.drive), "/delete".into(), "/y".into()])
}

#[tauri::command]
fn mount(app: AppHandle, id: String) -> Result<(), String> {
    let r = mount_conn(&app, &id);
    refresh_tray(&app);
    r
}

#[tauri::command]
fn unmount(app: AppHandle, id: String) -> Result<(), String> {
    let r = unmount_conn(&app, &id);
    refresh_tray(&app);
    r
}

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[tauri::command]
fn get_autostart() -> bool {
    run_cmd("reg", &["query".into(), RUN_KEY.into(), "/v".into(), "SftpDrive".into()]).is_ok()
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(err)?;
        let value = format!("\"{}\" --hidden", exe.display());
        run_cmd("reg", &["add".into(), RUN_KEY.into(), "/v".into(), "SftpDrive".into(), "/t".into(), "REG_SZ".into(), "/d".into(), value, "/f".into()])
    } else if get_autostart() {
        run_cmd("reg", &["delete".into(), RUN_KEY.into(), "/v".into(), "SftpDrive".into(), "/f".into()])
    } else {
        Ok(())
    }
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for c in load(app).unwrap_or_default() {
        let verb = if is_mounted(&c.drive) { "Desmontar" } else { "Montar" };
        let label = format!("{} {} ({}:)", verb, c.name, c.drive);
        menu.append(&MenuItem::with_id(app, format!("toggle:{}", c.id), label, true, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Abrir SftpDrive", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?)?;
    Ok(menu)
}

fn refresh_tray(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

// Mounts connections flagged "auto mount"; at login the network may not be up yet, so retry for a while.
fn auto_mount(app: AppHandle) {
    for _ in 0..6 {
        let pending: Vec<String> = load(&app)
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.auto_mount && !is_mounted(&c.drive))
            .map(|c| c.id)
            .collect();
        if pending.is_empty() {
            break;
        }
        for id in pending {
            let _ = mount_conn(&app, &id);
        }
        refresh_tray(&app);
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

#[tauri::command]
fn check_prereqs() -> bool {
    [r"C:\Program Files\SSHFS-Win\bin\sshfs-win.exe", r"C:\Program Files (x86)\SSHFS-Win\bin\sshfs-win.exe"]
        .iter()
        .any(|p| Path::new(p).exists())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app)))
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            // closing the window keeps the app (and the mounts) alive in the tray
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let menu = build_menu(&handle)?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().ok_or("sin icono")?.clone())
                .tooltip("SftpDrive")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_window(app),
                    "quit" => app.exit(0),
                    other => {
                        if let Some(id) = other.strip_prefix("toggle:") {
                            let (app, id) = (app.clone(), id.to_string());
                            std::thread::spawn(move || {
                                let drive_mounted = load(&app)
                                    .unwrap_or_default()
                                    .iter()
                                    .any(|c| c.id == id && is_mounted(&c.drive));
                                let _ = if drive_mounted { unmount_conn(&app, &id) } else { mount_conn(&app, &id) };
                                refresh_tray(&app);
                            });
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            if !std::env::args().any(|a| a == "--hidden") {
                show_window(&handle);
            }
            // keep the tray labels (Montar/Desmontar) in sync with drives mounted from elsewhere
            let h = handle.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(15));
                refresh_tray(&h);
            });
            std::thread::spawn(move || auto_mount(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_connections,
            save_connection,
            delete_connection,
            mount,
            unmount,
            check_prereqs,
            get_autostart,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar SftpDrive");
}
