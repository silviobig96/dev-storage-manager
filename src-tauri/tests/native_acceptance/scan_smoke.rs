//! Real Wry bridge + the production factory; only controlled temporary roots.
use super::*;
use std::{os::unix::fs::MetadataExt, time::Instant};

fn root_fingerprint(root: &std::path::Path) -> QaResult<Vec<(u64, u64, i64, i64)>> {
    [
        root.to_path_buf(),
        root.join("nested"),
        root.join("sentinel"),
        root.join("nested/data"),
    ]
    .iter()
    .map(|path| {
        let m = std::fs::symlink_metadata(path).map_err(transport)?;
        Ok((m.ino(), m.len(), m.mtime(), m.mtime_nsec()))
    })
    .collect()
}

pub(super) async fn run(fixture: &Fixture) -> QaResult<NativeEvidence> {
    let main = fixture.main()?;
    ready(&main).await?;
    let root = fixture
        .service
        .list_workspaces()
        .map_err(transport)?
        .workspaces
        .into_iter()
        .find(|r| r.id.get() == 1)
        .expect("explicit fixture ID 1")
        .selected_path;
    let root = std::path::Path::new(&root);
    let fingerprint = root_fingerprint(root)?;
    for (command, args) in [
        ("list_workspaces", json!({})),
        ("set_workspace_enabled", json!({"id":"1","enabled":false})),
        ("set_workspace_enabled", json!({"id":"1","enabled":true})),
        ("check_workspace", json!({"id":"1"})),
    ] {
        let value = invoke(&main, command, args, "scan-smoke").await?;
        require(
            value["status"] == "resolved",
            format!("fixture setup {command}: {value}"),
        )?;
        require(
            fixture
                .service
                .current_scan()
                .map_err(transport)?
                .session
                .is_none(),
            "load/add/list/enabled/check must not start scanning",
        )?;
    }
    let before = fixture.snapshot()?;
    let mut measurements = Vec::new();
    for (number, mode) in [(1, "quick"), (2, "deep")] {
        let current = invoke(&main, "get_workspace_scan", json!({}), "scan-smoke").await?;
        require(current["status"] == "resolved", "current scan status")?;
        let started_at = Instant::now();
        let started = invoke(
            &main,
            "start_workspace_scan",
            json!({"request":{
                "requestId":format!("{number:08x}-1111-1111-1111-111111111111"),
                "expectedEpoch":current["value"]["epoch"], "workspaceIds":["1"], "mode":mode
            }}),
            "scan-smoke",
        )
        .await?;
        require(
            started["status"] == "resolved",
            format!("real metadata start: {started}"),
        )?;
        let session = started["value"]["sessionId"].clone();
        let done = tokio::time::timeout(DEADLINE, async {
            loop {
                let value = invoke(
                    &main,
                    "get_workspace_scan_session",
                    json!({"sessionId":session}),
                    "scan-smoke",
                )
                .await?;
                require(value["status"] == "resolved", "real metadata session")?;
                if value["value"]["workerActive"] == false {
                    return Ok(value["value"].clone());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .map_err(|_| QaFailure::Timeout("real metadata worker"))??;
        let elapsed = started_at.elapsed().as_millis();
        require(
            done["state"] == "completed"
                && done["mode"] == mode
                && done["finishedAt"].is_string()
                && done["stopReason"].is_null(),
            format!("real terminal metadata: {done}"),
        )?;
        require(
            done["entryAttempts"] == "5"
                && done["metrics"]["observedLogicalBytes"] == "14"
                && done["metrics"]["uniqueFiles"] == "2"
                && done["metrics"]["uniqueDirectories"] == "2"
                && done["coverage"] == "complete_within_policy"
                && done["workspaces"].as_array().map(Vec::len) == Some(1)
                && done["workspaces"][0]["workspaceId"] == "1",
            "exact hand-counted metadata and explicit scope",
        )?;
        require(
            fixture.snapshot()? == before,
            "no persistence writes from scanning",
        )?;
        require(
            root_fingerprint(root)? == fingerprint
                && std::fs::read(root.join("sentinel")).map_err(transport)? == b"unchanged"
                && std::fs::read(root.join("nested/data")).map_err(transport)? == b"abcde"
                && std::fs::read_dir(root).map_err(transport)?.count() == 2
                && std::fs::read_dir(root.join("nested"))
                    .map_err(transport)?
                    .count()
                    == 1,
            "fixture and sentinel integrity",
        )?;
        measurements.push(json!({"mode":mode, "durationMillis":elapsed, "entryAttempts":done["entryAttempts"],
            "logicalBytes":done["metrics"]["observedLogicalBytes"], "allocatedBytes":done["metrics"]["observedAllocatedBytes"],
            "files":done["metrics"]["uniqueFiles"], "directories":done["metrics"]["uniqueDirectories"],
            "state":done["state"], "stopReason":done["stopReason"], "workerActive":done["workerActive"], "sentinel":"unchanged"}));
    }
    let reopened = crate::native_runtime::application_service(
        Database::open(&fixture.database_path).map_err(transport)?,
    );
    require(
        reopened
            .current_scan()
            .map_err(transport)?
            .session
            .is_none(),
        "restart must not resume a scan",
    )?;
    require(
        reopened
            .health()
            .map_err(transport)?
            .database_schema_version
            == 2,
        "schema 2 after scans/restart",
    )?;
    require(
        fixture.snapshot()? == before,
        "restart retained database bytes",
    )?;
    Ok(
        json!({"case":"scan-smoke", "runtime":"actual Wry", "runner":"production MetadataScanRunner",
        "workspaceIds":["1"], "measurements":measurements, "database":"identical", "schema":2,
        "noImplicitStart":["load","add","list","enabled","check","restart"],
        "shutdown":"ExitRequested checked after event loop"}),
    )
}
