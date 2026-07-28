use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use windows_sys::Win32::Foundation::GetLastError;
#[cfg(windows)]
use windows_sys::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetOption, WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
    WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_ADDREQ_FLAG_REPLACE, WINHTTP_FLAG_SECURE,
    WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

const DEFAULT_REPO: &str = "unixwin/winuxsh";
const USER_AGENT: &str = concat!("winuxsh/", env!("CARGO_PKG_VERSION"));
const HTTP_TIMEOUT_MS: i32 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfUpdateOptions {
    pub check: bool,
    pub dry_run: bool,
    pub force: bool,
    pub repo: String,
}

impl Default for SelfUpdateOptions {
    fn default() -> Self {
        Self {
            check: false,
            dry_run: false,
            force: false,
            repo: DEFAULT_REPO.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub fn run(args: &[String]) -> Result<()> {
    let options = parse_options(args)?;
    let release = fetch_latest_release(&options.repo)?;
    let current_tag = format!("v{}", env!("CARGO_PKG_VERSION"));

    if !options.force && !release_is_newer(&release.tag_name, &current_tag) {
        if release.tag_name == current_tag {
            println!("Winuxsh is already up to date ({current_tag})");
        } else {
            println!(
                "Current Winuxsh {current_tag} is newer than latest published release {}",
                release.tag_name
            );
        }
        return Ok(());
    }

    let arch = release_arch();
    println!(
        "Latest Winuxsh release: {} ({})",
        release.tag_name, release.html_url
    );

    let Some(asset) = select_installer_asset(&release.assets, arch) else {
        let portable = select_portable_asset(&release.assets, arch)
            .map(|asset| asset.browser_download_url.as_str())
            .unwrap_or(&release.html_url);
        if options.check {
            println!("Installer: unavailable for {arch}");
            println!("Portable package: {portable}");
            return Ok(());
        }
        anyhow::bail!(
            "latest release {} has no {} installer asset; portable package: {}",
            release.tag_name,
            arch,
            portable
        );
    };

    println!("Installer: {}", asset.name);

    if options.check {
        return Ok(());
    }

    let installer_path = download_asset(&options.repo, &release.tag_name, asset)?;
    println!("Downloaded: {}", installer_path.display());

    if options.dry_run {
        return Ok(());
    }

    if !cfg!(windows) {
        anyhow::bail!("self-update installer execution is only supported on Windows");
    }

    Command::new(&installer_path)
        .args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/CLOSEAPPLICATIONS",
        ])
        .spawn()
        .with_context(|| format!("start installer {}", installer_path.display()))?;

    println!("Started installer. Winuxsh will finish updating after this process exits.");
    Ok(())
}

fn parse_options(args: &[String]) -> Result<SelfUpdateOptions> {
    let mut options = SelfUpdateOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => options.check = true,
            "--dry-run" => options.dry_run = true,
            "--force" => options.force = true,
            "--repo" => {
                i += 1;
                let Some(repo) = args.get(i) else {
                    anyhow::bail!("--repo requires owner/name");
                };
                options.repo = repo.clone();
            }
            unknown => anyhow::bail!("unknown --self-update option '{}'", unknown),
        }
        i += 1;
    }
    Ok(options)
}

fn fetch_latest_release(repo: &str) -> Result<GitHubRelease> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let bytes = http_get_bytes(&url)?;
    serde_json::from_slice::<GitHubRelease>(&bytes).with_context(|| format!("parse {url}"))
}

fn download_asset(repo: &str, tag: &str, asset: &GitHubAsset) -> Result<PathBuf> {
    let dir = std::env::temp_dir()
        .join("winuxsh-self-update")
        .join(tag.trim_start_matches('v'));
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(safe_asset_name(&asset.name));

    let bytes = http_get_bytes(&asset.browser_download_url).with_context(|| {
        format!(
            "download {}/{} from {}",
            repo,
            asset.name,
            tag.trim_start_matches('v')
        )
    })?;
    std::fs::write(&path, bytes)
        .with_context(|| format!("write downloaded installer {}", path.display()))?;
    Ok(path)
}

#[cfg(windows)]
fn http_get_bytes(url: &str) -> Result<Vec<u8>> {
    let parsed = ParsedUrl::parse(url)?;
    let user_agent = wide_null(USER_AGENT);
    let host = wide_null(&parsed.host);
    let path = wide_null(&parsed.path);
    let get = wide_null("GET");

    let session = WinHttpHandle::new(
        unsafe {
            WinHttpOpen(
                user_agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            )
        },
        "WinHttpOpen",
    )?;

    if unsafe {
        WinHttpSetTimeouts(
            session.raw(),
            HTTP_TIMEOUT_MS,
            HTTP_TIMEOUT_MS,
            HTTP_TIMEOUT_MS,
            HTTP_TIMEOUT_MS,
        )
    } == 0
    {
        return Err(winhttp_error("WinHttpSetTimeouts"));
    }

    let connect = WinHttpHandle::new(
        unsafe { WinHttpConnect(session.raw(), host.as_ptr(), parsed.port, 0) },
        "WinHttpConnect",
    )?;

    let flags = if parsed.secure {
        WINHTTP_FLAG_SECURE
    } else {
        0
    };
    let request = WinHttpHandle::new(
        unsafe {
            WinHttpOpenRequest(
                connect.raw(),
                get.as_ptr(),
                path.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                flags,
            )
        },
        "WinHttpOpenRequest",
    )?;

    let redirect_policy = WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS;
    if unsafe {
        WinHttpSetOption(
            request.raw(),
            WINHTTP_OPTION_REDIRECT_POLICY,
            &redirect_policy as *const u32 as *const c_void,
            size_of::<u32>() as u32,
        )
    } == 0
    {
        return Err(winhttp_error("WinHttpSetOption redirect policy"));
    }

    let headers = wide_null(&request_headers());
    if unsafe {
        WinHttpAddRequestHeaders(
            request.raw(),
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
        )
    } == 0
    {
        return Err(winhttp_error("WinHttpAddRequestHeaders"));
    }

    if unsafe { WinHttpSendRequest(request.raw(), ptr::null(), 0, ptr::null(), 0, 0, 0) } == 0 {
        return Err(winhttp_error("WinHttpSendRequest"));
    }

    if unsafe { WinHttpReceiveResponse(request.raw(), ptr::null_mut()) } == 0 {
        return Err(winhttp_error("WinHttpReceiveResponse"));
    }

    let status = query_status_code(request.raw())?;
    if !(200..300).contains(&status) {
        let detail = read_response_body(request.raw())
            .map(|body| response_error_detail(&body))
            .unwrap_or_default();
        if detail.is_empty() {
            anyhow::bail!("HTTP status {status} for {url}");
        }
        anyhow::bail!("HTTP status {status} for {url}: {detail}");
    }

    read_response_body(request.raw())
}

#[cfg(not(windows))]
fn http_get_bytes(_url: &str) -> Result<Vec<u8>> {
    anyhow::bail!("self-update downloads require Windows WinHTTP")
}

#[cfg(windows)]
fn query_status_code(request: *mut c_void) -> Result<u32> {
    let mut status = 0_u32;
    let mut status_size = size_of::<u32>() as u32;
    let mut index = 0_u32;
    if unsafe {
        WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            &mut status as *mut u32 as *mut c_void,
            &mut status_size,
            &mut index,
        )
    } == 0
    {
        return Err(winhttp_error("WinHttpQueryHeaders status"));
    }
    Ok(status)
}

#[cfg(windows)]
fn read_response_body(request: *mut c_void) -> Result<Vec<u8>> {
    let mut data = Vec::new();

    loop {
        let mut available = 0_u32;
        if unsafe { WinHttpQueryDataAvailable(request, &mut available) } == 0 {
            return Err(winhttp_error("WinHttpQueryDataAvailable"));
        }
        if available == 0 {
            break;
        }

        let old_len = data.len();
        data.resize(old_len + available as usize, 0);

        let mut consumed = 0_u32;
        while consumed < available {
            let mut read = 0_u32;
            if unsafe {
                WinHttpReadData(
                    request,
                    data[old_len + consumed as usize..].as_mut_ptr() as *mut c_void,
                    available - consumed,
                    &mut read,
                )
            } == 0
            {
                return Err(winhttp_error("WinHttpReadData"));
            }
            if read == 0 {
                break;
            }
            consumed += read;
        }

        data.truncate(old_len + consumed as usize);
        if consumed == 0 {
            break;
        }
    }

    Ok(data)
}

#[cfg(windows)]
#[derive(Debug)]
struct ParsedUrl {
    secure: bool,
    host: String,
    port: u16,
    path: String,
}

#[cfg(windows)]
impl ParsedUrl {
    fn parse(url: &str) -> Result<Self> {
        let (secure, rest) = if let Some(rest) = url.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = url.strip_prefix("http://") {
            (false, rest)
        } else {
            anyhow::bail!("unsupported URL scheme in {url}");
        };

        let split_at = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..split_at];
        let path_tail = &rest[split_at..];
        let path_tail = path_tail
            .split_once('#')
            .map(|(path, _fragment)| path)
            .unwrap_or(path_tail);
        if authority.is_empty() || authority.contains('@') {
            anyhow::bail!("unsupported URL authority in {url}");
        }

        let (host, port) = split_host_port(authority, secure)?;
        let path = if path_tail.is_empty() {
            "/".to_string()
        } else if path_tail.starts_with('/') {
            path_tail.to_string()
        } else {
            format!("/{path_tail}")
        };

        Ok(Self {
            secure,
            host,
            port,
            path,
        })
    }
}

#[cfg(windows)]
fn split_host_port(authority: &str, secure: bool) -> Result<(String, u16)> {
    let default_port = if secure { 443 } else { 80 };

    if authority.starts_with('[') {
        let Some(end) = authority.find(']') else {
            anyhow::bail!("invalid IPv6 URL host");
        };
        let host = authority[..=end].to_string();
        let rest = &authority[end + 1..];
        let port = if let Some(raw) = rest.strip_prefix(':') {
            parse_port(raw)?
        } else if rest.is_empty() {
            default_port
        } else {
            anyhow::bail!("invalid URL authority");
        };
        return Ok((host, port));
    }

    if let Some((host, raw_port)) = authority.rsplit_once(':') {
        if raw_port.is_empty() || !raw_port.chars().all(|ch| ch.is_ascii_digit()) {
            anyhow::bail!("invalid URL port '{raw_port}'");
        }
        if host.is_empty() {
            anyhow::bail!("invalid URL host");
        }
        return Ok((host.to_string(), parse_port(raw_port)?));
    }

    Ok((authority.to_string(), default_port))
}

#[cfg(windows)]
fn parse_port(raw: &str) -> Result<u16> {
    raw.parse::<u16>()
        .with_context(|| format!("invalid URL port '{raw}'"))
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn request_headers() -> String {
    request_headers_for_token(github_token().as_deref())
}

fn request_headers_for_token(token: Option<&str>) -> String {
    let mut headers = format!(
        "User-Agent: {USER_AGENT}\r\nAccept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n"
    );
    if let Some(token) = token.and_then(clean_header_value) {
        headers.push_str("Authorization: Bearer ");
        headers.push_str(token);
        headers.push_str("\r\n");
    }
    headers
}

fn github_token() -> Option<String> {
    ["GH_TOKEN", "GITHUB_TOKEN"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| clean_header_value(value).is_some())
}

fn clean_header_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\r', '\n']) {
        None
    } else {
        Some(value)
    }
}

fn response_error_detail(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(300)
        .collect()
}

#[cfg(windows)]
fn winhttp_error(action: &str) -> anyhow::Error {
    let code = unsafe { GetLastError() };
    anyhow::anyhow!("{action} failed with Windows error {code}")
}

#[cfg(windows)]
struct WinHttpHandle(*mut c_void);

#[cfg(windows)]
impl WinHttpHandle {
    fn new(raw: *mut c_void, action: &str) -> Result<Self> {
        if raw.is_null() {
            Err(winhttp_error(action))
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> *mut c_void {
        self.0
    }
}

#[cfg(windows)]
impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}

fn safe_asset_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn select_installer_asset<'a>(assets: &'a [GitHubAsset], arch: &str) -> Option<&'a GitHubAsset> {
    let needle = format!("-win-{arch}-setup.exe");
    assets.iter().find(|asset| {
        let name = asset.name.to_ascii_lowercase();
        name.starts_with("winuxsh-v") && name.ends_with(&needle)
    })
}

fn select_portable_asset<'a>(assets: &'a [GitHubAsset], arch: &str) -> Option<&'a GitHubAsset> {
    let needle = format!("-win-{arch}.zip");
    assets.iter().find(|asset| {
        let name = asset.name.to_ascii_lowercase();
        name.starts_with("winuxsh-v") && name.ends_with(&needle)
    })
}

fn release_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "x64",
    }
}

fn release_is_newer(release_tag: &str, current_tag: &str) -> bool {
    let Some(release) = parse_version_tag(release_tag) else {
        return release_tag != current_tag;
    };
    let Some(current) = parse_version_tag(current_tag) else {
        return true;
    };
    release > current
}

fn parse_version_tag(tag: &str) -> Option<(u64, u64, u64)> {
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    let mut parts = tag.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{name}"),
        }
    }

    #[test]
    fn selects_matching_installer_asset() {
        let assets = vec![
            asset("winuxsh-v0.8.1-win-x64.zip"),
            asset("winuxsh-v0.8.1-win-x64-setup.exe"),
            asset("winuxsh-v0.8.1-win-arm64-setup.exe"),
        ];

        assert_eq!(
            select_installer_asset(&assets, "x64").map(|asset| asset.name.as_str()),
            Some("winuxsh-v0.8.1-win-x64-setup.exe")
        );
        assert_eq!(
            select_installer_asset(&assets, "arm64").map(|asset| asset.name.as_str()),
            Some("winuxsh-v0.8.1-win-arm64-setup.exe")
        );
    }

    #[test]
    fn sanitizes_download_file_names() {
        assert_eq!(safe_asset_name("../bad setup.exe"), ".._bad_setup.exe");
    }

    #[test]
    fn request_headers_include_github_rest_defaults() {
        let headers = request_headers_for_token(None);

        assert!(headers.contains("User-Agent: winuxsh/"));
        assert!(headers.contains("Accept: application/vnd.github+json"));
        assert!(headers.contains("X-GitHub-Api-Version: 2022-11-28"));
        assert!(!headers.contains("Authorization:"));
    }

    #[test]
    fn request_headers_can_include_clean_github_token() {
        let headers = request_headers_for_token(Some("  ghp_test  "));

        assert!(headers.contains("Authorization: Bearer ghp_test\r\n"));
    }

    #[test]
    fn request_headers_skip_header_injection_tokens() {
        let headers = request_headers_for_token(Some("good\r\nX-Bad: yes"));

        assert!(!headers.contains("Authorization:"));
    }

    #[test]
    fn response_error_detail_trims_and_limits_output() {
        let detail = response_error_detail(b"\n first line \nsecond line\nthird line\nfourth line");

        assert_eq!(detail, "first line second line third line");
    }

    #[cfg(windows)]
    #[test]
    fn parses_https_url_for_winhttp() {
        let parsed =
            ParsedUrl::parse("https://api.github.com/repos/unixwin/winuxsh/releases/latest?x=1")
                .unwrap();

        assert!(parsed.secure);
        assert_eq!(parsed.host, "api.github.com");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.path, "/repos/unixwin/winuxsh/releases/latest?x=1");
    }

    #[cfg(windows)]
    #[test]
    fn strips_url_fragment_before_winhttp_request() {
        let parsed = ParsedUrl::parse("https://example.com/downloads/app.exe?x=1#asset").unwrap();

        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.path, "/downloads/app.exe?x=1");
    }

    #[cfg(windows)]
    #[test]
    fn rejects_invalid_url_port() {
        assert!(ParsedUrl::parse("https://example.com:not-a-port/file").is_err());
    }

    #[test]
    fn compares_release_tags_without_downgrading() {
        assert!(release_is_newer("v0.8.2", "v0.8.1"));
        assert!(!release_is_newer("v0.8.1", "v0.8.1"));
        assert!(!release_is_newer("v0.8.0", "v0.8.1"));
    }
}
