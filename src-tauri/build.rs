fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_app_health",
            "list_workspaces",
            "select_and_add_workspace",
            "remove_workspace",
            "set_workspace_enabled",
            "check_workspace",
            "skip_workspace_setup",
            "start_workspace_scan",
            "get_workspace_scan",
            "get_workspace_scan_session",
            "cancel_workspace_scan",
        ]),
    ))
    .expect("failed to build application permissions")
}
