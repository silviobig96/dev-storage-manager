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
            sectionWidths:[...view.querySelectorAll('.content-section')].map(width),
            separatorWidths:[...view.querySelectorAll('.content-section > h2, .list-row + .list-row')].map(width),
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
    let window = main.window();
    let mut evidence = Vec::new();
    for (theme, dark) in [(tauri::Theme::Light, false), (tauri::Theme::Dark, true)] {
        window.set_theme(Some(theme)).map_err(transport)?;
        wait_for(
            &main,
            &format!("matchMedia('(prefers-color-scheme: dark)').matches === {dark}"),
            "per-window theme propagation",
        )
        .await?;
        for (width, height) in [(760., 520.), (1100., 720.), (1600., 900.)] {
            main.set_zoom(1.).map_err(transport)?;
            let mut requested = (width, height);
            let mut measured = None;
            for correction in 0..=2 {
                window
                    .set_size(tauri::LogicalSize::new(requested.0, requested.1))
                    .map_err(transport)?;
                tokio::time::sleep(Duration::from_millis(150)).await;
                let geometry = measure_css(&main).await?;
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
            require(
                !overview.empty_center_deltas.is_empty(),
                "Overview empty state must be measured",
            )?;
            // DOM activation is only for automatic geometry collection. Native
            // keyboard/pointer/focus interaction is a separate human/agent check.
            eval(&main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Workspaces').click()").await?;
            wait_for(
                &main,
                "document.querySelectorAll('.workspace-row').length === 12",
                "fixture workspace rows",
            )
            .await?;
            let workspaces = measure_css(&main).await?;
            assert_layout(&workspaces, dark)?;
            main.set_zoom(2.).map_err(transport)?;
            tokio::time::sleep(Duration::from_millis(150)).await;
            let native_zoom = page_zoom(&main).await?;
            require(
                native_zoom == 2.,
                format!("native pageZoom must be 2.0, got {native_zoom}"),
            )?;
            let zoomed = measure_css(&main).await?;
            require(
                zoomed.main_overflow == 0. && zoomed.document_overflow == 0.,
                format!("zoom 2 horizontal overflow: {zoomed:?}"),
            )?;
            main.set_zoom(1.).map_err(transport)?;
            require(
                page_zoom(&main).await? == 1.,
                "native pageZoom restoration must be 1.0",
            )?;
            evidence.push(json!({"theme":format!("{theme:?}"),"requestedCss":[width,height],"titlebarCorrections":corrections,
                "overview":overview,"workspaces":workspaces,"zoom2":{"nativePageZoom":native_zoom,"geometry":zoomed},"restoredNativePageZoom":1.0}));
            eval(&main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Overview').click()").await?;
            wait_for(
                &main,
                "!!document.querySelector('.overview-view')",
                "Overview navigation",
            )
            .await?;
        }
    }
    window.set_theme(None).map_err(transport)?;
    main.set_zoom(1.).map_err(transport)?;
    let result = json!({"case":"inherited-layout","matrix":evidence,"nativeInputQa":"AWAITING SEPARATE REVIEW","visualQa":"AWAITING HUMAN REVIEW"});
    eprintln!("AUTOMATIC_GEOMETRY {result}");
    if std::env::args().any(|arg| arg == "--hold") {
        eval(&main, "[...document.querySelectorAll('nav button')].find(b=>b.textContent==='Workspaces').click()").await?;
        main.set_zoom(2.).map_err(transport)?;
        tokio::time::sleep(Duration::from_millis(150)).await;
        let zoom = page_zoom(&main).await?;
        require(zoom == 2., "interactive native pageZoom must be 2.0")?;
        eprintln!("INTERACTIVE READY pid={} pageZoom={zoom} geometry={}: native_acceptance fixture; 10 minute native input review window. No personal database opened.",std::process::id(), serde_json::to_value(measure_css(&main).await?).map_err(transport)?);
        tokio::time::sleep(Duration::from_secs(600)).await;
        main.set_zoom(1.).map_err(transport)?;
        require(
            page_zoom(&main).await? == 1.,
            "interactive native pageZoom restoration",
        )?;
    }
    Ok(result)
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
