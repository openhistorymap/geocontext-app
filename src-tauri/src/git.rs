//! Git + GitHub support for the editor.
//!
//! - Clone / pull / push via libgit2 (vendored, no system git needed).
//! - GitHub access via OS keychain (`keyring` crate); the token is set
//!   either by the OAuth Device Flow or by browser-assisted PAT entry.
//! - Minimal GitHub REST calls via `ureq`.

use std::path::Path;

use git2::{
    Cred, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const KEYRING_SERVICE: &str = "geocontext-editor";
const KEYRING_USER: &str = "github";

const OAUTH_CLIENT_ID: Option<&str> = option_env!("GEOCONTEXT_OAUTH_CLIENT_ID");
const OAUTH_SCOPE: &str = "repo";

const UA: &str = concat!("geocontext-editor/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("github error: {0}")]
    Github(String),
    #[error("oauth client_id is not configured at build time")]
    NoOauthClient,
    #[error("not a git repository: {0}")]
    NotARepo(String),
    #[error("no remote configured")]
    NoRemote,
    #[error("authentication required but no token available")]
    NoToken,
    #[error("repository is not on a branch (detached HEAD)")]
    DetachedHead,
    #[error("non-fast-forward pull required \u{2014} resolve manually")]
    NonFastForward,
    #[error("{0}")]
    Other(String),
}

impl From<keyring::Error> for GitError {
    fn from(e: keyring::Error) -> Self {
        GitError::Keyring(e.to_string())
    }
}

impl From<ureq::Error> for GitError {
    fn from(e: ureq::Error) -> Self {
        GitError::Http(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Token storage

pub fn store_token(token: &str) -> Result<(), GitError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.set_password(token)?;
    Ok(())
}

pub fn load_token() -> Result<Option<String>, GitError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn clear_token() -> Result<(), GitError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Credentials callback adapter

fn callbacks_with_token(token: Option<String>) -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let Some(t) = token.as_deref() {
                // GitHub accepts any non-empty username when password is a token.
                return Cred::userpass_plaintext("x-access-token", t);
            }
        }
        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            return Cred::default();
        }
        if let Some(user) = username_from_url {
            return Cred::ssh_key_from_agent(user);
        }
        Err(git2::Error::from_str(
            "no credentials available — sign in via the Account menu",
        ))
    });
    cb
}

// ---------------------------------------------------------------------------
// Clone

#[derive(Debug, Serialize)]
pub struct CloneResult {
    pub folder: String,
    pub default_branch: Option<String>,
}

pub fn clone(url: &str, dest: &Path, token: Option<String>) -> Result<CloneResult, GitError> {
    if dest.exists()
        && dest
            .read_dir()
            .map(|mut r| r.next().is_some())
            .unwrap_or(false)
    {
        return Err(GitError::Other(format!(
            "destination {} already exists and is non-empty",
            dest.display()
        )));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cb = callbacks_with_token(token);
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);
    let repo = builder.clone(url, dest)?;
    let default = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(str::to_string));
    Ok(CloneResult {
        folder: dest.display().to_string(),
        default_branch: default,
    })
}

// ---------------------------------------------------------------------------
// Status

#[derive(Debug, Serialize)]
pub struct StatusResult {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub remote: Option<String>,
    pub remote_url: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub dirty: u32,
    pub head_sha: Option<String>,
}

pub fn status(folder: &Path) -> Result<StatusResult, GitError> {
    let repo = match Repository::open(folder) {
        Ok(r) => r,
        Err(_) => {
            return Ok(StatusResult {
                is_repo: false,
                branch: None,
                remote: None,
                remote_url: None,
                ahead: 0,
                behind: 0,
                dirty: 0,
                head_sha: None,
            })
        }
    };

    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand().map(str::to_string));
    let head_sha = head
        .as_ref()
        .and_then(|h| h.target())
        .map(|oid| oid.to_string());

    // Pick the first remote (typically "origin")
    let remotes = repo.remotes()?;
    let remote_name = remotes.iter().flatten().next().map(str::to_string);
    let remote_url = remote_name
        .as_ref()
        .and_then(|n| repo.find_remote(n).ok())
        .and_then(|r| r.url().map(str::to_string));

    let mut ahead = 0usize;
    let mut behind = 0usize;
    if let (Some(local_ref), Some(remote)) = (head.as_ref(), remote_name.as_ref()) {
        if let Some(local_oid) = local_ref.target() {
            if let Some(shorthand) = local_ref.shorthand() {
                let upstream_name = format!("{remote}/{shorthand}");
                if let Ok(upstream_ref) = repo.find_branch(&upstream_name, git2::BranchType::Remote)
                {
                    if let Some(remote_oid) = upstream_ref.get().target() {
                        if let Ok((a, b)) = repo.graph_ahead_behind(local_oid, remote_oid) {
                            ahead = a;
                            behind = b;
                        }
                    }
                }
            }
        }
    }

    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let dirty = statuses.iter().filter(|s| !s.status().is_empty()).count() as u32;

    Ok(StatusResult {
        is_repo: true,
        branch,
        remote: remote_name,
        remote_url,
        ahead,
        behind,
        dirty,
        head_sha,
    })
}

// ---------------------------------------------------------------------------
// Pull (fetch + fast-forward only)

pub fn pull(folder: &Path, token: Option<String>) -> Result<StatusResult, GitError> {
    let repo =
        Repository::open(folder).map_err(|_| GitError::NotARepo(folder.display().to_string()))?;
    let remotes = repo.remotes()?;
    let remote_name = remotes
        .iter()
        .flatten()
        .next()
        .map(str::to_string)
        .ok_or(GitError::NoRemote)?;

    let head = repo.head()?;
    let branch_name = head.shorthand().ok_or(GitError::DetachedHead)?.to_string();

    {
        let mut remote = repo.find_remote(&remote_name)?;
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(callbacks_with_token(token));
        remote.fetch(&[branch_name.as_str()], Some(&mut fo), None)?;
    }

    let upstream = format!("{remote_name}/{branch_name}");
    let upstream_ref = repo.find_branch(&upstream, git2::BranchType::Remote)?;
    let upstream_oid = upstream_ref
        .get()
        .target()
        .ok_or_else(|| GitError::Other("upstream has no target".into()))?;

    let local_oid = head
        .target()
        .ok_or_else(|| GitError::Other("HEAD has no target".into()))?;

    if local_oid == upstream_oid {
        return status(folder);
    }
    let analysis = repo.merge_analysis(&[&repo.find_annotated_commit(upstream_oid)?])?;
    if analysis.0.is_fast_forward() {
        let refname = format!("refs/heads/{branch_name}");
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(upstream_oid, "fast-forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        return status(folder);
    }
    Err(GitError::NonFastForward)
}

// ---------------------------------------------------------------------------
// Sync = commit all + push

pub fn sync(
    folder: &Path,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
    token: Option<String>,
) -> Result<StatusResult, GitError> {
    let repo =
        Repository::open(folder).map_err(|_| GitError::NotARepo(folder.display().to_string()))?;

    // Stage everything (including deletions).
    {
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
    }
    let oid = {
        let mut index = repo.index()?;
        index.write_tree()?
    };
    let tree = repo.find_tree(oid)?;

    let head = repo.head().ok();
    let parents: Vec<git2::Commit> = head
        .as_ref()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .map(|c| vec![c])
        .unwrap_or_default();

    let sig = author_signature(&repo, name, email)?;

    // Skip the commit if nothing was staged and there's already a parent.
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    let needs_commit = parents
        .first()
        .map(|p| p.tree_id() != tree.id())
        .unwrap_or(true);

    if needs_commit {
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;
    }

    // Push
    let remotes = repo.remotes()?;
    let remote_name = remotes
        .iter()
        .flatten()
        .next()
        .map(str::to_string)
        .ok_or(GitError::NoRemote)?;
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand().map(str::to_string))
        .or_else(|| Some("main".to_string()))
        .unwrap();

    {
        let mut remote = repo.find_remote(&remote_name)?;
        let mut po = PushOptions::new();
        po.remote_callbacks(callbacks_with_token(token));
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        remote.push(&[refspec.as_str()], Some(&mut po))?;
    }

    status(folder)
}

fn author_signature<'a>(
    repo: &'a Repository,
    name: Option<&str>,
    email: Option<&str>,
) -> Result<Signature<'a>, GitError> {
    if let (Some(n), Some(e)) = (name, email) {
        return Ok(Signature::now(n, e)?);
    }
    // Try repo config; fall back to a generic identity if absent.
    let config = repo.config().ok();
    let cfg_name = config.as_ref().and_then(|c| c.get_string("user.name").ok());
    let cfg_email = config
        .as_ref()
        .and_then(|c| c.get_string("user.email").ok());
    let n = name
        .map(str::to_string)
        .or(cfg_name)
        .unwrap_or_else(|| "GeoContext Editor".into());
    let e = email
        .map(str::to_string)
        .or(cfg_email)
        .unwrap_or_else(|| "noreply@openhistorymap.org".into());
    Ok(Signature::now(&n, &e)?)
}

// ---------------------------------------------------------------------------
// "Init local repo and add origin" — used by the publish-to-GitHub flow.

pub fn ensure_repo_with_origin(folder: &Path, origin_url: &str) -> Result<(), GitError> {
    let repo = match Repository::open(folder) {
        Ok(r) => r,
        Err(_) => Repository::init(folder)?,
    };
    match repo.find_remote("origin") {
        Ok(mut remote) => {
            if remote.url() != Some(origin_url) {
                repo.remote_set_url("origin", origin_url)?;
            }
            let _ = remote.connect(git2::Direction::Fetch); // best-effort
        }
        Err(_) => {
            repo.remote("origin", origin_url)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GitHub OAuth Device Flow

#[derive(Debug, Serialize)]
pub struct DeviceFlowStart {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PollResult {
    Pending,
    SlowDown {
        interval: u64,
    },
    Token {
        access_token: String,
        scope: Option<String>,
    },
    Denied,
    Expired,
    Error {
        message: String,
    },
}

pub fn oauth_client_id() -> Option<&'static str> {
    OAUTH_CLIENT_ID
}

#[derive(Deserialize)]
struct DeviceStartResp {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct AccessTokenResp {
    access_token: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

pub fn oauth_start() -> Result<DeviceFlowStart, GitError> {
    let client_id = OAUTH_CLIENT_ID.ok_or(GitError::NoOauthClient)?;
    let resp: DeviceStartResp = ureq::post("https://github.com/login/device/code")
        .set("Accept", "application/json")
        .set("User-Agent", UA)
        .send_form(&[("client_id", client_id), ("scope", OAUTH_SCOPE)])?
        .into_json()
        .map_err(|e| GitError::Http(e.to_string()))?;
    Ok(DeviceFlowStart {
        user_code: resp.user_code,
        device_code: resp.device_code,
        verification_uri: resp.verification_uri,
        expires_in: resp.expires_in,
        interval: resp.interval,
    })
}

pub fn oauth_poll(device_code: &str) -> Result<PollResult, GitError> {
    let client_id = OAUTH_CLIENT_ID.ok_or(GitError::NoOauthClient)?;
    let resp: AccessTokenResp = ureq::post("https://github.com/login/oauth/access_token")
        .set("Accept", "application/json")
        .set("User-Agent", UA)
        .send_form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])?
        .into_json()
        .map_err(|e| GitError::Http(e.to_string()))?;

    if let Some(token) = resp.access_token {
        return Ok(PollResult::Token {
            access_token: token,
            scope: resp.scope,
        });
    }
    match resp.error.as_deref() {
        Some("authorization_pending") => Ok(PollResult::Pending),
        Some("slow_down") => Ok(PollResult::SlowDown {
            interval: resp.interval.unwrap_or(5),
        }),
        Some("expired_token") => Ok(PollResult::Expired),
        Some("access_denied") => Ok(PollResult::Denied),
        Some(other) => Ok(PollResult::Error {
            message: other.to_string(),
        }),
        None => Ok(PollResult::Error {
            message: "empty response from GitHub".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Browser-assisted PAT — just build a token-create URL with prefilled scopes.

pub fn pat_create_url() -> String {
    let description = urlencoding::encode("GeoContext Editor");
    let scopes = urlencoding::encode("repo,workflow");
    format!("https://github.com/settings/tokens/new?description={description}&scopes={scopes}")
}

// ---------------------------------------------------------------------------
// GitHub REST API

#[derive(Debug, Serialize)]
pub struct WhoAmI {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Deserialize)]
struct UserResp {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

pub fn whoami(token: &str) -> Result<WhoAmI, GitError> {
    let resp = ureq::get("https://api.github.com/user")
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", UA)
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()?;
    let scopes = resp
        .header("X-OAuth-Scopes")
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let body: UserResp = resp
        .into_json()
        .map_err(|e| GitError::Http(e.to_string()))?;
    Ok(WhoAmI {
        login: body.login,
        name: body.name,
        avatar_url: body.avatar_url,
        scopes,
    })
}

#[derive(Debug, Serialize)]
pub struct CreatedRepo {
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub full_name: String,
}

#[derive(Deserialize)]
struct CreateRepoResp {
    html_url: String,
    clone_url: String,
    ssh_url: String,
    default_branch: String,
    full_name: String,
}

pub fn create_repo(
    token: &str,
    name: &str,
    description: Option<&str>,
    is_private: bool,
    owner_org: Option<&str>,
) -> Result<CreatedRepo, GitError> {
    let url = match owner_org {
        Some(org) => format!("https://api.github.com/orgs/{org}/repos"),
        None => "https://api.github.com/user/repos".to_string(),
    };
    let body = ureq::json!({
        "name": name,
        "description": description.unwrap_or(""),
        "private": is_private,
        "auto_init": false,
    });
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", UA)
        .set("X-GitHub-Api-Version", "2022-11-28")
        .send_json(body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => {
            let msg = r
                .into_string()
                .unwrap_or_else(|_| "GitHub returned an error".into());
            return Err(GitError::Github(msg));
        }
        Err(e) => return Err(GitError::Http(e.to_string())),
    };
    let r: CreateRepoResp = resp
        .into_json()
        .map_err(|e| GitError::Http(e.to_string()))?;
    Ok(CreatedRepo {
        html_url: r.html_url,
        clone_url: r.clone_url,
        ssh_url: r.ssh_url,
        default_branch: r.default_branch,
        full_name: r.full_name,
    })
}

// ---------------------------------------------------------------------------
// URL helpers

/// Accept "owner/repo", "https://github.com/owner/repo", "git@github.com:owner/repo.git",
/// etc.; return a canonical https clone URL.
pub fn normalise_github_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return s.to_string();
    }
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        let trimmed = rest.trim_end_matches(".git");
        return format!("https://github.com/{trimmed}");
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        return s.trim_end_matches(".git").to_string();
    }
    // bare "owner/repo"
    if s.matches('/').count() == 1 && !s.contains(' ') {
        return format!("https://github.com/{s}");
    }
    s.to_string()
}
