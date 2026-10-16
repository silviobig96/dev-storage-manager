use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    width: f64,
    height: f64,
    dark: bool,
    main_overflow: f64,
    document_overflow: f64,
    content_width: f64,
    view_width: f64,
    section_widths: Vec<f64>,
    separator_widths: Vec<f64>,
    empty_center_deltas: Vec<f64>,
    clipped_paths: usize,
    controls_overflow: f64,
    workspace_rows: usize,
}

// Regression caught: sizing/content padding or layout changes that overflow the
// actual WK viewport, misalign sections, or stop respecting per-window themes.
pub async fn measure_css(webview: &Webview<Wry>) -> QaResult<Geometry> {
    let value = eval(webview, r#"(() => {
        const main = document.querySelector('main');
        const view = main?.firstElementChild;
        if (!main || !view) return {error:'production frontend not ready'};
        const width = element => element.getBoundingClientRect().width;
        const style = getComputedStyle(main);
        const contentWidth = main.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
        return {width:innerWidth,height:innerHeight,dark:matchMedia('(prefers-color-scheme: dark)').matches,
            mainOverflow: main.scrollWidth-main.clientWidth, documentOverflow:document.documentElement.scrollWidth-innerWidth,
            contentWidth,viewWidth:width(view),
            clippedPaths:[...view.querySelectorAll('.workspace-path')].filter(e=>e.scrollWidth>e.clientWidth+1 || e.scrollHeight>e.clientHeight+1).length,
            controlsOverflow:Math.max(0,...[...view.querySelectorAll('.scan-controls,.workspace-actions')].map(e=>e.scrollWidth-e.clientWidth)),
            workspaceRows:view.querySelectorAll('.workspace-row').length,
            sectionWidths:[...view.querySelectorAll('.content-section')].map(width),
            separatorWidths:[...view.querySelectorAll('.content-section > h2, .content-section > .data-list > .list-row + .list-row')].map(width),
            emptyCenterDeltas:[...view.querySelectorAll('.empty-state')].flatMap(empty => {
                const content=view.getBoundingClientRect();
                return [empty,...empty.querySelectorAll('h3,p')].map(element => {
                    const box=element.getBoundingClientRect();
                    return Math.abs((content.left+content.right-box.left-box.right)/2);
                });
            })};
    })()"#).await?;
    serde_json::from_value(value).map_err(transport)
}

pub(super) async fn run(fixture: &Fixture) -> QaResult<Value> {
    let main = fixture.main()?;
    ready(&main).await?;
    let populated = populated_matrix(&main).await?;
    for row in fixture
        .service
        .list_workspaces()
        .map_err(transport)?
        .workspaces
    {
        let result = invoke(
            &main,
            "remove_workspace",
            json!({"id":row.id.get().to_string()}),
            "empty-layout",
        )
        .await?;
        require(
            result["status"] == "resolved",
            "remove only fixture configuration",
        )?;
    }
    eval(&main, "location.reload()").await?;
    wait_for(&main, "document.querySelector('.app-shell') && document.querySelectorAll('.workspace-row').length===0", "empty fixture reload").await?;
    // Also require fresh frontend's empty Workspaces before collecting the matrix.
    eval(&main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Workspaces').click()").await?;
    wait_for(
        &main,
        "document.querySelector('.workspaces-view .empty-state')!==null",
        "empty Workspaces state",
    )
    .await?;
    let empty = populated_matrix(&main).await?;
    for row in empty["matrix"].as_array().unwrap() {
        require(
            row["workspaces"]["workspaceRows"] == 0
                && !row["workspaces"]["emptyCenterDeltas"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
            "actual empty Workspaces in every matrix cell",
        )?;
    }
    Ok(
        json!({"case":"inherited-layout","populated":populated,"empty":empty,"activation":"DOM layout collection; native input unverified","visualQa":"AWAITING HUMAN REVIEW"}),
    )
}

pub(super) async fn populated_matrix(main: &Webview) -> QaResult<Value> {
    let window = main.window();
    let original_size = window.inner_size().map_err(transport)?;
    let original_zoom = page_zoom(main).await?;
    let outcome = matrix(main).await;
    window.unmaximize().map_err(transport)?;
    window.set_theme(None).map_err(transport)?;
    main.set_zoom(original_zoom).map_err(transport)?;
    window.set_size(original_size).map_err(transport)?;
    require(
        page_zoom(main).await? == original_zoom,
        "native pageZoom restored",
    )?;
    outcome
}

async fn matrix(main: &Webview) -> QaResult<Value> {
    let window = main.window();
    let mut evidence = Vec::new();
    for (theme, dark) in [(tauri::Theme::Light, false), (tauri::Theme::Dark, true)] {
        window.set_theme(Some(theme)).map_err(transport)?;
        wait_for(
            main,
            &format!("matchMedia('(prefers-color-scheme: dark)').matches === {dark}"),
            "per-window theme propagation",
        )
        .await?;
        for (width, height) in [(760., 520.), (1100., 720.), (1600., 900.)] {
            eval(main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Overview').click()").await?;
            wait_for(
                main,
                "!!document.querySelector('.overview-view')",
                "Overview",
            )
            .await?;
            main.set_zoom(1.).map_err(transport)?;
            let mut requested = (width, height);
            let mut measured = None;
            for correction in 0..=2 {
                window
                    .set_size(tauri::LogicalSize::new(requested.0, requested.1))
                    .map_err(transport)?;
                tokio::time::sleep(Duration::from_millis(150)).await;
                let geometry = measure_css(main).await?;
                if geometry.width == width && geometry.height == height {
                    measured = Some((geometry, correction));
                    break;
                }
                requested.0 += width - geometry.width;
                requested.1 += height - geometry.height;
            }
            let (overview, corrections) = measured.ok_or_else(|| {
                QaFailure::Assertion(format!(
                    "exact CSS {width}x{height} unavailable after two titlebar corrections"
                ))
            })?;
            assert_layout(&overview, dark)?;
            let overview_zoom = zoom_measure(main).await?;
            // DOM activation is only for automatic geometry collection. Native
            // keyboard/pointer/focus interaction is a separate human/agent check.
            eval(main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Workspaces').click()").await?;
            wait_for(
                main,
                "!!document.querySelector('.workspaces-view')",
                "fixture Workspaces",
            )
            .await?;
            let workspaces = measure_css(main).await?;
            assert_layout(&workspaces, dark)?;
            let zoomed = zoom_measure(main).await?;
            evidence.push(json!({"theme":format!("{theme:?}"),"requestedCss":[width,height],"titlebarCorrections":corrections,
                "overview":overview,"overviewZoom2":overview_zoom,"workspaces":workspaces,"zoom2":zoomed,"restoredNativePageZoom":1.0}));
            eval(main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Overview').click()").await?;
            wait_for(
                main,
                "!!document.querySelector('.overview-view')",
                "Overview navigation",
            )
            .await?;
        }
    }
    window.set_theme(None).map_err(transport)?;
    main.set_zoom(1.).map_err(transport)?;
    window.maximize().map_err(transport)?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    require(
        window.is_maximized().map_err(transport)?,
        "native maximize state",
    )?;
    let mut maximized = Vec::new();
    for view in ["Overview", "Workspaces"] {
        eval(
            main,
            format!(
                "[...document.querySelectorAll('nav button')].find(b=>b.textContent==={}).click()",
                serde_json::to_string(view).unwrap()
            ),
        )
        .await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
        let normal = measure_css(main).await?;
        require(
            normal.main_overflow == 0. && normal.document_overflow == 0.,
            "maximized overflow",
        )?;
        maximized.push(json!({"view":view,"geometry":normal,"zoom2":zoom_measure(main).await?}));
    }
    window.unmaximize().map_err(transport)?;
    let result = json!({"matrix":evidence,"maximized":maximized,"nativeInputQa":"AWAITING SEPARATE REVIEW","visualQa":"AWAITING HUMAN REVIEW"});
    eprintln!("AUTOMATIC_GEOMETRY {result}");
    Ok(result)
}

async fn zoom_measure(main: &Webview) -> QaResult<Value> {
    main.set_zoom(2.).map_err(transport)?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let native = page_zoom(main).await?;
    require(native == 2., "actual WK pageZoom 2")?;
    let measured = measure_css(main).await?;
    require(
        measured.main_overflow == 0.
            && measured.document_overflow == 0.
            && measured.clipped_paths == 0
            && measured.controls_overflow <= 1.,
        format!("zoom layout {measured:?}"),
    )?;
    main.set_zoom(1.).map_err(transport)?;
    require(page_zoom(main).await? == 1., "zoom restoration")?;
    Ok(json!({"nativePageZoom":native,"geometry":measured,"restoredNativePageZoom":1.0}))
}

fn assert_layout(geometry: &Geometry, dark: bool) -> QaResult<()> {
    require(
        geometry.dark == dark,
        "native appearance must propagate to prefers-color-scheme",
    )?;
    require(
        geometry.main_overflow == 0. && geometry.document_overflow == 0.,
        format!("horizontal overflow: {geometry:?}"),
    )?;
    require(
        geometry.clipped_paths == 0 && geometry.controls_overflow <= 1.,
        "full paths and controls fit",
    )?;
    require(
        !geometry.section_widths.is_empty() && !geometry.separator_widths.is_empty(),
        "missing sections/separators",
    )?;
    for width in std::iter::once(&geometry.view_width)
        .chain(geometry.section_widths.iter())
        .chain(geometry.separator_widths.iter())
    {
        require(
            (width - geometry.content_width).abs() <= 1.,
            format!("content-width alignment: {geometry:?}"),
        )?;
    }
    for delta in &geometry.empty_center_deltas {
        require(*delta <= 1., format!("empty-state center delta {delta}"))?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
async fn page_zoom(webview: &Webview) -> QaResult<f64> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    webview
        .with_webview(move |platform| {
            // Tauri supplies the actual WKWebView on its owning main thread. The
            // borrowed pointer stays inside this closure; no ownership is assumed.
            let view = unsafe { &*platform.inner().cast::<objc2_web_kit::WKWebView>() };
            let _ = tx.send(unsafe { view.pageZoom() });
        })
        .map_err(transport)?;
    tokio::time::timeout(DEADLINE, rx)
        .await
        .map_err(|_| QaFailure::Timeout("native pageZoom getter"))?
        .map_err(transport)
}

#[cfg(not(target_os = "macos"))]
async fn page_zoom(_: &Webview) -> QaResult<f64> {
    Err(QaFailure::Transport(
        "native acceptance requires macOS WKWebView".into(),
    ))
}
