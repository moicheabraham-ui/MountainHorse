#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

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

fn run_net(args: &[String]) -> Result<(), String> {
    let mut cmd = Command::new("net");
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
    save(&app, &list)
}

#[tauri::command]
fn delete_connection(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load(&app)?;
    list.retain(|c| c.id != id);
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &id) {
        let _ = entry.delete_credential();
    }
    save(&app, &list)
}

#[tauri::command]
fn mount(app: AppHandle, id: String) -> Result<(), String> {
    let c = load(&app)?.into_iter().find(|c| c.id == id).ok_or("Conexión no encontrada")?;
    if is_mounted(&c.drive) {
        return Err(format!("La unidad {}: ya está en uso.", c.drive));
    }
    // \\sshfs.r = path from the server root, \\sshfs.k = key auth (uses %USERPROFILE%\.ssh)
    let prefix = if c.use_key { "sshfs.k" } else { "sshfs.r" };
    let path = c.remote_path.trim().replace('/', "\\");
    let path = if path.is_empty() || path.starts_with('\\') { path } else { format!("\\{}", path) };
    let unc = format!("\\\\{}\\{}@{}!{}{}", prefix, c.user, c.host, c.port, path);

    let mut args = vec!["use".to_string(), format!("{}:", c.drive), unc];
    if !c.use_key {
        let pw = keyring::Entry::new(KEYRING_SERVICE, &c.id)
            .and_then(|e| e.get_password())
            .map_err(|_| "Falta la contraseña: edita la conexión y guárdala.".to_string())?;
        args.push(pw);
    }
    args.push(format!("/user:{}", c.user));
    args.push("/persistent:no".into());
    run_net(&args)
}

#[tauri::command]
fn unmount(app: AppHandle, id: String) -> Result<(), String> {
    let c = load(&app)?.into_iter().find(|c| c.id == id).ok_or("Conexión no encontrada")?;
    run_net(&["use".into(), format!("{}:", c.drive), "/delete".into(), "/y".into()])
}

#[tauri::command]
fn check_prereqs() -> bool {
    [r"C:\Program Files\SSHFS-Win\bin\sshfs-win.exe", r"C:\Program Files (x86)\SSHFS-Win\bin\sshfs-win.exe"]
        .iter()
        .any(|p| Path::new(p).exists())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_connections,
            save_connection,
            delete_connection,
            mount,
            unmount,
            check_prereqs
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar SftpDrive");
}
