use openstruct_distribution::updater::{data_root, Installation, Snapshot, Updater};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};

pub struct Updates(pub Result<Arc<Updater>, String>);
impl Updates {
    pub fn new() -> Self {
        Self(
            (|| {
                // A failed installation detection must not hide the update check:
                // fall back to a non-installable mode that still consults the
                // catalogue and only gives guidance (no installer is executed).
                let installation =
                    Installation::detect(cfg!(debug_assertions)).or_else(|e| {
                        eprintln!(
                            "Instalação não identificada; atualização interna desabilitada: {e:#}"
                        );
                        unidentified_installation()
                    })?;
                Updater::new(
                    data_root()?,
                    installation,
                    env!("CARGO_PKG_VERSION").into(),
                )
            })()
            .map_err(|e| format!("{e:#}")),
        )
    }
    fn service(&self, window: &WebviewWindow) -> Result<Arc<Updater>, String> {
        if window.label() != "main" {
            return Err("Janela não autorizada".into());
        }
        self.0.clone()
    }
}
/// Same guidance as a system installation: consult only, update by the original method.
fn unidentified_installation() -> anyhow::Result<Installation> {
    Ok(Installation {
        mode: "system".into(),
        executable: std::env::current_exe()?,
        managed_root: None,
    })
}
#[tauri::command]
pub fn updates_snapshot(
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Snapshot, String> {
    Ok(state.service(&window)?.snapshot())
}
#[tauri::command]
pub async fn updates_check(
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Snapshot, String> {
    Ok(state.service(&window)?.check().await)
}
#[tauri::command]
pub async fn updates_prepare(
    version: String,
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Snapshot, String> {
    state
        .service(&window)?
        .prepare(&version)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub fn updates_cancel(
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Snapshot, String> {
    state
        .service(&window)?
        .cancel()
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub async fn updates_arm(
    version: String,
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<Snapshot, String> {
    state
        .service(&window)?
        .arm(&version)
        .await
        .map_err(|e| format!("{e:#}"))
}
#[tauri::command]
pub fn updates_finish(
    version: String,
    window: WebviewWindow,
    state: State<'_, Updates>,
) -> Result<(), String> {
    state
        .service(&window)?
        .authorize_exit(&version)
        .map_err(|e| format!("{e:#}"))?;
    window.app_handle().exit(0);
    Ok(())
}
pub fn cli() -> bool {
    let mut args = std::env::args_os().skip(1);
    match args.next().and_then(|s| s.into_string().ok()).as_deref() {
        Some("--openstruct-product-info") => {
            println!(
                "{}",
                serde_json::json!({"product":"armadurastruct","version":env!("CARGO_PKG_VERSION")})
            );
            true
        }
        Some("--openstruct-update-helper") => {
            let result = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("Pedido ausente"))
                .and_then(|p| {
                    anyhow::ensure!(args.next().is_none(), "Argumentos excedentes");
                    openstruct_distribution::updater::run_helper(std::path::Path::new(&p))
                });
            if let Err(e) = result {
                eprintln!("Atualização não concluída: {e:#}");
                std::process::exit(2)
            }
            true
        }
        _ => false,
    }
}
