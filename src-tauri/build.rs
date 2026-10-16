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
        ]),
    ))
    .expect("failed to build application permissions")
}
