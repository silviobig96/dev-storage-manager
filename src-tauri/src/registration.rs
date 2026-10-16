pub(crate) fn application_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

pub(crate) fn application_handler<R: tauri::Runtime>(
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::commands::get_app_health,
        crate::workspaces::list_workspaces,
        crate::workspaces::select_and_add_workspace,
        crate::workspaces::remove_workspace,
        crate::workspaces::set_workspace_enabled,
        crate::workspaces::check_workspace,
        crate::workspaces::skip_workspace_setup
    ]
}
