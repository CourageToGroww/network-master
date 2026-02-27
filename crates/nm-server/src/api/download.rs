use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use axum_extra::extract::Host;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/download", get(landing_page))
        .route("/download/agent", get(download_agent))
        .route("/download/install.ps1", get(install_script))
}

/// Returns true if an agent binary has been uploaded.
async fn binary_exists(state: &AppState) -> bool {
    let path = state.update_dir.join("nm-agent.exe");
    tokio::fs::metadata(&path).await.is_ok()
}

/// GET /download — HTML landing page with instructions + download button + one-liner
async fn landing_page(
    State(state): State<AppState>,
    Host(host): Host,
) -> impl IntoResponse {
    let server_addr = host_to_server_addr(&host);
    let has_binary = binary_exists(&state).await;

    let (status_banner, download_section) = if has_binary {
        (
            r#"<div style="background:#15803d;color:#fff;padding:12px 20px;border-radius:6px;margin-bottom:24px">Agent binary ready for download.</div>"#.to_string(),
            format!(
                r##"
        <h2>Option 1: PowerShell One-Liner</h2>
        <p>Open <strong>PowerShell as Administrator</strong> on the target PC and paste:</p>
        <div style="background:#1e1e1e;color:#d4d4d4;padding:16px;border-radius:6px;font-family:monospace;font-size:14px;overflow-x:auto;position:relative">
            <span id="cmd">irm http://{server_addr}/download/install.ps1 | iex</span>
            <button onclick="navigator.clipboard.writeText(document.getElementById('cmd').textContent).then(function(){{this.textContent='Copied!';var b=this;setTimeout(function(){{b.textContent='Copy'}},2000)}}.bind(this))" style="position:absolute;right:12px;top:12px;background:#333;color:#fff;border:1px solid #555;padding:4px 12px;border-radius:4px;cursor:pointer;font-size:12px">Copy</button>
        </div>

        <h2 style="margin-top:32px">Option 2: Manual Download</h2>
        <p>Download the installer, then run as Administrator:</p>
        <a href="/download/agent" style="display:inline-block;background:#2563eb;color:#fff;padding:10px 24px;border-radius:6px;text-decoration:none;font-weight:600;margin:8px 0">Download nm-agent.exe</a>
        <pre style="background:#1e1e1e;color:#d4d4d4;padding:16px;border-radius:6px;margin-top:12px">nm-agent.exe install --server {server_addr}</pre>
"##,
                server_addr = server_addr
            ),
        )
    } else {
        (
            r#"<div style="background:#dc2626;color:#fff;padding:12px 20px;border-radius:6px;margin-bottom:24px">No agent binary uploaded yet. Upload one via the dashboard or API first.</div>"#.to_string(),
            String::new(),
        )
    };

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Network Master - Agent Install</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #e2e8f0; padding: 40px 20px; }}
        .container {{ max-width: 680px; margin: 0 auto; }}
        h1 {{ font-size: 24px; margin-bottom: 8px; }}
        h2 {{ font-size: 18px; margin-bottom: 8px; color: #94a3b8; }}
        p {{ line-height: 1.6; margin-bottom: 12px; color: #cbd5e1; }}
        pre {{ overflow-x: auto; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Network Master Agent</h1>
        <p style="margin-bottom:24px;color:#94a3b8">Install the monitoring agent on any Windows PC.</p>
        {status_banner}
        {download_section}
    </div>
</body>
</html>"##,
        status_banner = status_banner,
        download_section = download_section,
    );

    Html(html)
}

/// GET /download/agent — serve the raw nm-agent.exe binary
async fn download_agent(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let path = state.update_dir.join("nm-agent.exe");

    let data = tokio::fs::read(&path).await.map_err(|_| {
        StatusCode::NOT_FOUND
    })?;

    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/octet-stream",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"nm-agent.exe\"",
            ),
        ],
        data,
    ))
}

/// GET /download/install.ps1 — PowerShell install script with server address baked in
async fn install_script(
    State(state): State<AppState>,
    Host(host): Host,
) -> Result<impl IntoResponse, StatusCode> {
    if !binary_exists(&state).await {
        return Err(StatusCode::NOT_FOUND);
    }

    let server_addr = host_to_server_addr(&host);

    let script = format!(
        r#"#Requires -RunAsAdministrator
# Network Master Agent Installer
# Generated by server at {server_addr}

$ErrorActionPreference = 'Stop'
$agentUrl = 'http://{server_addr}/download/agent'
$tmpExe = "$env:TEMP\nm-agent.exe"

Write-Host 'Network Master Agent Installer' -ForegroundColor Cyan
Write-Host '================================' -ForegroundColor Cyan
Write-Host ''

# Download
Write-Host "[1/3] Downloading agent from $agentUrl ..."
Invoke-WebRequest -Uri $agentUrl -OutFile $tmpExe -UseBasicParsing

# Install
Write-Host '[2/3] Installing agent ...'
& $tmpExe install --server {server_addr}

# Cleanup
Write-Host '[3/3] Cleaning up ...'
Remove-Item $tmpExe -Force -ErrorAction SilentlyContinue

Write-Host ''
Write-Host 'Done! Agent installed and running.' -ForegroundColor Green
Write-Host 'It should appear in the dashboard within a few seconds.'
"#,
        server_addr = server_addr,
    );

    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            ),
        ],
        script,
    ))
}

/// Extract a usable server address from the Host header.
/// Strips port 80 (default HTTP) but keeps non-standard ports.
fn host_to_server_addr(host: &str) -> String {
    if host.ends_with(":80") {
        host.trim_end_matches(":80").to_string()
    } else {
        host.to_string()
    }
}
