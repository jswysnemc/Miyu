use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use tokio::process::Command;

const GIT_DIFF_MAX_BYTES: usize = 512 * 1024;
const GIT_LOG_DEFAULT_LIMIT: usize = 50;
const GIT_LOG_MAX_LIMIT: usize = 200;

/// 旧版兼容：单个 Git 文件状态。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
}

/// 旧版兼容：当前工作区 Git 状态与 Diff。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GitDiff {
    pub repository: bool,
    pub branch: String,
    pub status: String,
    pub files: Vec<GitFileStatus>,
    pub diff: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct GitDirtyCounts {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitStatusEntry {
    pub path: String,
    pub old_path: Option<String>,
    pub index_status: String,
    pub worktree_status: String,
    pub kind: String,
    pub staged: bool,
    pub conflicted: bool,
    pub untracked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitRepositoryState {
    pub repo_root: String,
    pub workdir: String,
    pub head: String,
    pub upstream: String,
    pub remote_name: String,
    pub remote_url: String,
    pub ahead: i32,
    pub behind: i32,
    pub stash_count: i32,
    pub dirty_counts: GitDirtyCounts,
    pub entries: Vec<GitStatusEntry>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitBranch {
    pub name: String,
    pub full_name: String,
    pub kind: String,
    pub current: bool,
    pub upstream: String,
    pub ahead: i32,
    pub behind: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitBranchesResponse {
    pub state: GitRepositoryState,
    pub branches: Vec<GitBranch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitDiffResponse {
    pub base_ref: String,
    pub head_ref: String,
    pub mode: String,
    pub files: Vec<String>,
    pub patch: String,
    pub stat: String,
    pub truncated: bool,
    pub binary_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitCommitFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitCommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub parents: Vec<String>,
    pub refs: Vec<String>,
    pub subject: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: String,
    pub files: Vec<GitCommitFile>,
    pub file_count: usize,
    pub local_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitLogResponse {
    pub state: GitRepositoryState,
    pub commits: Vec<GitCommitSummary>,
    pub history_base_ref: String,
    pub history_remote_ref: String,
    pub history_ahead: i32,
    pub history_behind: i32,
    pub merge_base: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitCommitDetails {
    pub sha: String,
    pub short_sha: String,
    pub subject: String,
    pub body: String,
    pub author_name: String,
    pub author_email: String,
    pub author_date: String,
    pub files: Vec<GitCommitFile>,
    pub file_count: usize,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub stat: String,
    pub remote_name: String,
    pub remote_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitCommitDetailsResponse {
    pub state: GitRepositoryState,
    pub commit: GitCommitDetails,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GitOperationResponse {
    pub ok: bool,
    pub state: GitRepositoryState,
    pub stdout: String,
    pub stderr: String,
    pub message: String,
}

struct GitOutput {
    stdout: String,
    stderr: String,
}

/// 读取旧版兼容 Diff。
pub(crate) async fn read_git_diff(root: &Path) -> Result<GitDiff> {
    let state = git_status(root).await?;
    if state.status != "ready" {
        return Ok(empty_git_diff());
    }
    let patch = match git_diff(root, "working_tree", None).await {
        Ok(value) => value.patch,
        Err(_) => String::new(),
    };
    Ok(GitDiff {
        repository: true,
        branch: state.head.clone(),
        status: format!("## {}", state.head),
        files: state
            .entries
            .iter()
            .map(|entry| GitFileStatus {
                path: entry.path.clone(),
                index_status: entry.index_status.replace('.', " "),
                worktree_status: entry.worktree_status.replace('.', " "),
            })
            .collect(),
        diff: patch,
    })
}

/// 执行旧版兼容 Git 操作。
pub(crate) async fn apply_git_action(
    root: &Path,
    action: &str,
    paths: &[String],
    message: Option<&str>,
) -> Result<GitDiff> {
    match action {
        "init" => {
            let branch = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("main");
            run_git(root, &["init", "-b", branch]).await?;
        }
        "stage" => {
            if paths.is_empty() {
                git_op(root, "stage_all", None, None, None).await?;
            } else {
                for path in paths {
                    git_op(root, "stage", Some(path.as_str()), None, None).await?;
                }
            }
        }
        "unstage" => {
            if paths.is_empty() {
                git_op(root, "unstage_all", None, None, None).await?;
            } else {
                for path in paths {
                    git_op(root, "unstage", Some(path.as_str()), None, None).await?;
                }
            }
        }
        "discard" => {
            if paths.is_empty() {
                bail!("discard requires at least one path");
            }
            for path in paths {
                git_op(root, "discard", Some(path.as_str()), None, None).await?;
            }
        }
        "commit" => {
            git_op(root, "commit", None, message, None).await?;
        }
        _ => bail!("unsupported git action: {action}"),
    }
    read_git_diff(root).await
}

/// 读取仓库状态。
pub(crate) async fn git_status(root: &Path) -> Result<GitRepositoryState> {
    let workdir = root.display().to_string();
    let Some(repo_root) = discover_repo(root).await? else {
        return Ok(not_repo_state(&workdir));
    };
    let output = git_raw(
        &repo_root,
        &["status", "--porcelain=v2", "--branch", "--show-stash", "-z"],
    )
    .await?;
    if !output.status.success() {
        return Ok(GitRepositoryState {
            repo_root: repo_root.display().to_string(),
            workdir,
            head: String::new(),
            upstream: String::new(),
            remote_name: String::new(),
            remote_url: String::new(),
            ahead: 0,
            behind: 0,
            stash_count: 0,
            dirty_counts: GitDirtyCounts::default(),
            entries: Vec::new(),
            status: "error".to_string(),
            error: Some(trim_bytes(&output.stderr)),
        });
    }
    let (head, upstream, ahead, behind, stash_count, entries) =
        parse_status_porcelain_v2(&output.stdout);
    let (remote_name, remote_url) = resolve_state_remote(&repo_root, &upstream).await;
    Ok(GitRepositoryState {
        repo_root: repo_root.display().to_string(),
        workdir,
        head,
        upstream,
        remote_name,
        remote_url,
        ahead,
        behind,
        stash_count,
        dirty_counts: dirty_counts(&entries),
        entries,
        status: "ready".to_string(),
        error: None,
    })
}

/// 读取分支列表。
pub(crate) async fn git_branches(root: &Path) -> Result<GitBranchesResponse> {
    let state = git_status(root).await?;
    if state.status != "ready" {
        return Ok(GitBranchesResponse {
            state,
            branches: Vec::new(),
        });
    }
    let mut branches = Vec::new();
    let local = git_success(
        Path::new(&state.repo_root),
        &[
            "for-each-ref",
            "--format=%(refname:short)%00%(upstream:short)%00%(HEAD)",
            "refs/heads",
        ],
    )
    .await?;
    for line in local.stdout.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\0').collect();
        if parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        let name = parts[0].to_string();
        let upstream = parts.get(1).unwrap_or(&"").to_string();
        let current = parts.get(2).map(|value| *value == "*").unwrap_or(false);
        let (ahead, behind) = if current {
            (state.ahead, state.behind)
        } else {
            (0, 0)
        };
        branches.push(GitBranch {
            name: name.clone(),
            full_name: name,
            kind: "local".to_string(),
            current,
            upstream,
            ahead,
            behind,
        });
    }
    let remote = git_success(
        Path::new(&state.repo_root),
        &["for-each-ref", "--format=%(refname:short)", "refs/remotes"],
    )
    .await
    .unwrap_or_else(|_| empty_output());
    for name in remote.stdout.lines() {
        let name = name.trim();
        if name.is_empty() || name.ends_with("/HEAD") {
            continue;
        }
        branches.push(GitBranch {
            name: name.to_string(),
            full_name: name.to_string(),
            kind: "remote".to_string(),
            current: false,
            upstream: String::new(),
            ahead: 0,
            behind: 0,
        });
    }
    if !state.head.is_empty()
        && state.head != "(detached)"
        && !branches
            .iter()
            .any(|branch| branch.kind == "local" && branch.full_name == state.head)
    {
        branches.insert(
            0,
            GitBranch {
                name: state.head.clone(),
                full_name: state.head.clone(),
                kind: "local".to_string(),
                current: true,
                upstream: state.upstream.clone(),
                ahead: state.ahead,
                behind: state.behind,
            },
        );
    }
    Ok(GitBranchesResponse { state, branches })
}

/// 读取提交历史。
pub(crate) async fn git_log(
    root: &Path,
    limit: Option<usize>,
    skip: Option<usize>,
) -> Result<GitLogResponse> {
    let state = git_status(root).await?;
    if state.status != "ready" || !ref_exists(Path::new(&state.repo_root), "HEAD").await {
        return Ok(GitLogResponse {
            state,
            commits: Vec::new(),
            history_base_ref: String::new(),
            history_remote_ref: String::new(),
            history_ahead: 0,
            history_behind: 0,
            merge_base: String::new(),
        });
    }
    let limit = limit
        .unwrap_or(GIT_LOG_DEFAULT_LIMIT)
        .clamp(1, GIT_LOG_MAX_LIMIT);
    let skip = skip.unwrap_or(0);
    let mut args = vec![
        "log".to_string(),
        "--date=iso-strict".to_string(),
        "--decorate=short".to_string(),
        "--topo-order".to_string(),
        "--parents".to_string(),
        "--name-status".to_string(),
        "-z".to_string(),
        "--find-renames".to_string(),
        format!("--max-count={limit}"),
        "--pretty=format:%x1e%H%x1f%h%x1f%P%x1f%D%x1f%an%x1f%ae%x1f%aI%x1f%s".to_string(),
    ];
    if skip > 0 {
        args.push(format!("--skip={skip}"));
    }
    let remote_ref = if state.upstream.trim().is_empty() {
        String::new()
    } else {
        state.upstream.clone()
    };
    let mut history_ahead = state.ahead;
    let mut history_behind = state.behind;
    let mut merge_base = String::new();
    if !remote_ref.is_empty() {
        if let Ok(output) = git_success(
            Path::new(&state.repo_root),
            &["merge-base", "HEAD", remote_ref.as_str()],
        )
        .await
        {
            merge_base = output.stdout.trim().to_string();
        }
        if let Ok(output) = git_success(
            Path::new(&state.repo_root),
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("HEAD...{remote_ref}"),
            ],
        )
        .await
        {
            let mut parts = output.stdout.split_whitespace();
            history_ahead = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            history_behind = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
        }
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git_success(Path::new(&state.repo_root), &arg_refs).await?;
    let commits = parse_git_log(&output.stdout);
    Ok(GitLogResponse {
        state,
        commits,
        history_base_ref: merge_base.clone(),
        history_remote_ref: remote_ref,
        history_ahead,
        history_behind,
        merge_base,
    })
}

/// 读取提交详情。
pub(crate) async fn git_commit_details(
    root: &Path,
    commit: &str,
) -> Result<GitCommitDetailsResponse> {
    let state = ensure_ready(root).await?;
    let commit = commit.trim();
    if commit.is_empty() {
        bail!("commit sha cannot be empty");
    }
    let metadata = git_success(
        Path::new(&state.repo_root),
        &[
            "show",
            "-s",
            "--date=iso-strict",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%b",
            commit,
        ],
    )
    .await?;
    let fields: Vec<&str> = metadata.stdout.splitn(7, '\x1f').collect();
    if fields.len() < 7 {
        bail!("unable to parse commit details");
    }
    let files_output = git_success(
        Path::new(&state.repo_root),
        &[
            "show",
            "--format=",
            "--name-status",
            "-z",
            "--find-renames",
            commit,
        ],
    )
    .await?;
    let files = parse_name_status_records(&files_output.stdout);
    let stat = git_success(
        Path::new(&state.repo_root),
        &["show", "--format=", "--stat", "--find-renames", commit],
    )
    .await?;
    let shortstat = git_success(
        Path::new(&state.repo_root),
        &["show", "--format=", "--shortstat", "--find-renames", commit],
    )
    .await?;
    let (files_changed, insertions, deletions) = parse_shortstat(&shortstat.stdout);
    Ok(GitCommitDetailsResponse {
        commit: GitCommitDetails {
            sha: fields[0].trim().to_string(),
            short_sha: fields[1].trim().to_string(),
            author_name: fields[2].trim().to_string(),
            author_email: fields[3].trim().to_string(),
            author_date: fields[4].trim().to_string(),
            subject: fields[5].trim().to_string(),
            body: fields[6].trim().to_string(),
            file_count: files.len(),
            files,
            files_changed,
            insertions,
            deletions,
            stat: stat.stdout.trim().to_string(),
            remote_name: state.remote_name.clone(),
            remote_url: state.remote_url.clone(),
        },
        state,
    })
}

/// 读取工作树/分支 Diff。
pub(crate) async fn git_diff(
    root: &Path,
    mode: &str,
    path: Option<&str>,
) -> Result<GitDiffResponse> {
    let state = ensure_ready(root).await?;
    let clean_path = path.map(validate_repo_relative_path).transpose()?;
    let files: Vec<String> = if let Some(path) = clean_path.clone() {
        vec![path]
    } else {
        state.entries.iter().map(|entry| entry.path.clone()).collect()
    };
    if mode == "working_tree" {
        return working_tree_diff(&state, files, clean_path.as_deref()).await;
    }

    let base_ref = if !state.upstream.trim().is_empty() {
        state.upstream.clone()
    } else {
        resolve_default_base(Path::new(&state.repo_root))
            .await
            .unwrap_or_default()
    };
    if base_ref.is_empty() {
        bail!("找不到可用于审查的基线分支。请先设置 upstream 或 fetch 主分支。");
    }
    let mut args = vec![
        "diff".to_string(),
        "--patch".to_string(),
        "--stat".to_string(),
        format!("{base_ref}...HEAD"),
    ];
    if let Some(path) = clean_path.as_deref() {
        args.push("--".to_string());
        args.push(path.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git_success(Path::new(&state.repo_root), &arg_refs).await?;
    let (stat, patch) = split_stat_and_patch(&output.stdout);
    let (patch, truncated) = truncate_patch(patch);
    Ok(GitDiffResponse {
        base_ref,
        head_ref: "HEAD".to_string(),
        mode: mode.to_string(),
        files,
        patch,
        stat,
        truncated,
        binary_files: Vec::new(),
    })
}

/// 读取提交 Diff。
pub(crate) async fn git_commit_diff(
    root: &Path,
    commit: &str,
    path: Option<&str>,
) -> Result<GitDiffResponse> {
    let state = ensure_ready(root).await?;
    let commit = commit.trim();
    if commit.is_empty() {
        bail!("commit sha cannot be empty");
    }
    let clean_path = path.map(validate_repo_relative_path).transpose()?;
    let parent_output =
        git_success(Path::new(&state.repo_root), &["show", "-s", "--format=%P", commit]).await?;
    let first_parent = parent_output
        .stdout
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let mut args: Vec<String> = if first_parent.is_empty() {
        vec![
            "show".to_string(),
            "--format=".to_string(),
            "--patch".to_string(),
            "--stat".to_string(),
            "--find-renames".to_string(),
            commit.to_string(),
        ]
    } else {
        vec![
            "diff".to_string(),
            "--patch".to_string(),
            "--stat".to_string(),
            "--find-renames".to_string(),
            first_parent.clone(),
            commit.to_string(),
        ]
    };
    if let Some(path) = clean_path.as_deref() {
        args.push("--".to_string());
        args.push(path.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git_success(Path::new(&state.repo_root), &arg_refs).await?;
    let (stat, patch) = split_stat_and_patch(&output.stdout);
    let (patch, truncated) = truncate_patch(patch);
    Ok(GitDiffResponse {
        base_ref: if first_parent.is_empty() {
            "ROOT".to_string()
        } else {
            first_parent
        },
        head_ref: commit.to_string(),
        mode: "commit".to_string(),
        files: clean_path.into_iter().collect(),
        patch,
        stat,
        truncated,
        binary_files: Vec::new(),
    })
}

/// 执行写操作。
pub(crate) async fn git_op(
    root: &Path,
    action: &str,
    path: Option<&str>,
    message: Option<&str>,
    remote_url: Option<&str>,
) -> Result<GitOperationResponse> {
    if action == "init" {
        let branch = message
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("main");
        let result = run_git_output(root, &["init", "-b", branch]).await;
        return operation_response(root, result, "仓库已初始化。").await;
    }

    let state = ensure_ready(root).await?;
    let repo = Path::new(&state.repo_root);
    let result = match action {
        "stage" => {
            let path = validate_repo_relative_path(path.unwrap_or_default())?;
            git_success(repo, &["add", "--", path.as_str()]).await
        }
        "stage_all" => git_success(repo, &["add", "-A", "--"]).await,
        "unstage" => {
            let path = validate_repo_relative_path(path.unwrap_or_default())?;
            if !ref_exists(repo, "HEAD").await {
                git_success(repo, &["rm", "--cached", "--", path.as_str()]).await
            } else {
                git_success(repo, &["restore", "--staged", "--", path.as_str()]).await
            }
        }
        "unstage_all" => {
            if !ref_exists(repo, "HEAD").await {
                if state.dirty_counts.staged > 0 {
                    git_success(repo, &["rm", "--cached", "-r", "--", "."]).await
                } else {
                    Ok(empty_output())
                }
            } else {
                git_success(repo, &["restore", "--staged", "--", "."]).await
            }
        }
        "discard" => {
            let path = validate_repo_relative_path(path.unwrap_or_default())?;
            let is_untracked = state
                .entries
                .iter()
                .any(|entry| entry.path == path && entry.untracked);
            if is_untracked {
                git_success(repo, &["clean", "-fd", "--", path.as_str()]).await
            } else if !ref_exists(repo, "HEAD").await {
                git_success(repo, &["rm", "-f", "--", path.as_str()]).await
            } else {
                git_success(repo, &["restore", "--staged", "--worktree", "--", path.as_str()]).await
            }
        }
        "discard_all" => {
            if !ref_exists(repo, "HEAD").await {
                let remove = if state.dirty_counts.staged > 0 {
                    git_success(repo, &["rm", "-f", "-r", "--", "."]).await?
                } else {
                    empty_output()
                };
                let clean = git_success(repo, &["clean", "-fd", "--", "."]).await?;
                Ok(merge_outputs([remove, clean]))
            } else {
                let restore =
                    git_success(repo, &["restore", "--staged", "--worktree", "--", "."]).await?;
                let clean = git_success(repo, &["clean", "-fd", "--", "."]).await?;
                Ok(merge_outputs([restore, clean]))
            }
        }
        "commit" => {
            let message = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("commit message cannot be empty"))?;
            if state.dirty_counts.staged == 0 {
                bail!("没有已暂存的改动可提交。");
            }
            git_success(repo, &["commit", "-m", message]).await
        }
        "fetch" => {
            if git_remote_names(repo).await?.is_empty() {
                Err(anyhow::anyhow!("当前仓库还没有设置远端仓库。"))
            } else {
                git_success(repo, &["fetch", "--prune"]).await
            }
        }
        "pull" => pull_repo(repo, &state).await,
        "push" => push_repo(repo, &state).await,
        "set_remote" => {
            let remote_url = remote_url
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("远端仓库地址不能为空。"))?;
            if git_origin_exists(repo).await {
                git_success(repo, &["remote", "set-url", "origin", remote_url]).await
            } else {
                git_success(repo, &["remote", "add", "origin", remote_url]).await
            }
        }
        "switch_branch" => switch_branch(repo, message).await,
        "create_branch" => {
            let branch = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("branch name cannot be empty"))?;
            git_success(repo, &["switch", "-c", branch]).await
        }
        "add_to_gitignore" => add_to_gitignore(repo, path).await,
        "stash_push" => {
            let mut args = vec!["stash", "push", "--include-untracked"];
            let owned;
            if let Some(value) = message.map(str::trim).filter(|value| !value.is_empty()) {
                owned = value.to_string();
                args.extend(["-m", owned.as_str()]);
            }
            git_success(repo, &args).await
        }
        "stash_pop" => git_success(repo, &["stash", "pop"]).await,
        _ => bail!("unsupported git action: {action}"),
    };
    let message = match action {
        "stage" | "stage_all" => "文件已暂存。",
        "unstage" | "unstage_all" => "文件已取消暂存。",
        "discard" | "discard_all" => "改动已放弃。",
        "commit" => "提交已创建。",
        "fetch" => "Fetch 完成。",
        "pull" => "Pull 完成。",
        "push" => "Push 完成。",
        "set_remote" => "远端仓库已保存。",
        "switch_branch" => "分支已切换。",
        "create_branch" => "分支已创建。",
        "add_to_gitignore" => "路径已添加到 .gitignore。",
        "stash_push" => "改动已贮藏。",
        "stash_pop" => "贮藏已弹出。",
        _ => "操作完成。",
    };
    operation_response(root, result, message).await
}

async fn working_tree_diff(
    state: &GitRepositoryState,
    files: Vec<String>,
    clean_path: Option<&str>,
) -> Result<GitDiffResponse> {
    if !ref_exists(Path::new(&state.repo_root), "HEAD").await {
        let mut patch = String::new();
        for entry in &state.entries {
            if let Some(path) = clean_path {
                if entry.path != path {
                    continue;
                }
            }
            if entry.untracked {
                if let Ok(Some(part)) =
                    build_untracked_file_patch(Path::new(&state.repo_root), &entry.path).await
                {
                    if !patch.is_empty() {
                        patch.push('\n');
                    }
                    patch.push_str(&part);
                }
            }
        }
        let (patch, truncated) = truncate_patch(patch);
        return Ok(GitDiffResponse {
            base_ref: "ROOT".to_string(),
            head_ref: "WORKTREE".to_string(),
            mode: "working_tree".to_string(),
            files,
            patch,
            stat: String::new(),
            truncated,
            binary_files: Vec::new(),
        });
    }
    let mut args = vec![
        "diff".to_string(),
        "--patch".to_string(),
        "--stat".to_string(),
        "HEAD".to_string(),
    ];
    if let Some(path) = clean_path {
        args.push("--".to_string());
        args.push(path.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git_success(Path::new(&state.repo_root), &arg_refs).await?;
    let (stat, mut patch) = split_stat_and_patch(&output.stdout);
    for entry in &state.entries {
        if !entry.untracked {
            continue;
        }
        if let Some(path) = clean_path {
            if entry.path != path {
                continue;
            }
        }
        if let Ok(Some(part)) =
            build_untracked_file_patch(Path::new(&state.repo_root), &entry.path).await
        {
            if !patch.is_empty() {
                patch.push('\n');
            }
            patch.push_str(&part);
        }
    }
    let (patch, truncated) = truncate_patch(patch);
    Ok(GitDiffResponse {
        base_ref: "HEAD".to_string(),
        head_ref: "WORKTREE".to_string(),
        mode: "working_tree".to_string(),
        files,
        patch,
        stat,
        truncated,
        binary_files: Vec::new(),
    })
}

async fn pull_repo(repo: &Path, state: &GitRepositoryState) -> Result<GitOutput> {
    if state.upstream.trim().is_empty() {
        if state.head.trim().is_empty() || state.head == "(detached)" {
            Err(anyhow::anyhow!("当前不在可拉取的本地分支上。"))
        } else if !git_origin_exists(repo).await {
            Err(anyhow::anyhow!(
                "当前分支没有 upstream，且找不到 origin remote。"
            ))
        } else {
            git_success(repo, &["pull", "--ff-only", "origin", state.head.as_str()]).await
        }
    } else {
        git_success(repo, &["pull", "--ff-only"]).await
    }
}

async fn push_repo(repo: &Path, state: &GitRepositoryState) -> Result<GitOutput> {
    if state.upstream.trim().is_empty() {
        if state.head.trim().is_empty() || state.head == "(detached)" {
            Err(anyhow::anyhow!("当前不在可推送的本地分支上。"))
        } else if !git_origin_exists(repo).await {
            Err(anyhow::anyhow!(
                "当前分支没有 upstream，且找不到 origin remote。"
            ))
        } else {
            git_success(repo, &["push", "-u", "origin", state.head.as_str()]).await
        }
    } else {
        git_success(repo, &["push"]).await
    }
}

async fn switch_branch(repo: &Path, message: Option<&str>) -> Result<GitOutput> {
    let branch = message
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("branch name cannot be empty"))?;
    if branch.starts_with("origin/")
        || (branch.contains('/') && remote_ref_exists(repo, branch).await)
    {
        let local = branch.rsplit('/').next().unwrap_or(branch);
        if branch_exists_local(repo, local).await {
            git_success(repo, &["switch", local]).await
        } else {
            git_success(repo, &["switch", "-c", local, "--track", branch]).await
        }
    } else {
        git_success(repo, &["switch", branch]).await
    }
}

async fn add_to_gitignore(repo: &Path, path: Option<&str>) -> Result<GitOutput> {
    let path = validate_repo_relative_path(path.unwrap_or_default())?;
    let pattern = format!("/{path}");
    let gitignore = repo.join(".gitignore");
    let mut content = match tokio::fs::read_to_string(&gitignore).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => bail!("读取 .gitignore 失败：{error}"),
    };
    let already = content.lines().any(|line| {
        let line = line.trim();
        line == path || line == pattern
    });
    if !already {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&pattern);
        content.push('\n');
        tokio::fs::write(&gitignore, content).await?;
    }
    Ok(empty_output())
}

async fn operation_response(
    root: &Path,
    result: Result<GitOutput>,
    message: &str,
) -> Result<GitOperationResponse> {
    match result {
        Ok(output) => Ok(GitOperationResponse {
            ok: true,
            state: git_status(root).await?,
            stdout: output.stdout,
            stderr: output.stderr,
            message: message.to_string(),
        }),
        Err(error) => Ok(GitOperationResponse {
            ok: false,
            state: git_status(root)
                .await
                .unwrap_or_else(|_| not_repo_state(&root.display().to_string())),
            stdout: String::new(),
            stderr: error.to_string(),
            message: error.to_string(),
        }),
    }
}

async fn ensure_ready(root: &Path) -> Result<GitRepositoryState> {
    let state = git_status(root).await?;
    if state.status != "ready" {
        bail!(state
            .error
            .unwrap_or_else(|| "当前目录不是 Git 仓库。".to_string()));
    }
    Ok(state)
}

async fn discover_repo(root: &Path) -> Result<Option<std::path::PathBuf>> {
    let output = git_raw(root, &["rev-parse", "--show-toplevel"]).await?;
    if !output.status.success() {
        return Ok(None);
    }
    let text = trim_bytes(&output.stdout);
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(std::path::PathBuf::from(text)))
}

fn not_repo_state(workdir: &str) -> GitRepositoryState {
    GitRepositoryState {
        repo_root: String::new(),
        workdir: workdir.to_string(),
        head: String::new(),
        upstream: String::new(),
        remote_name: String::new(),
        remote_url: String::new(),
        ahead: 0,
        behind: 0,
        stash_count: 0,
        dirty_counts: GitDirtyCounts::default(),
        entries: Vec::new(),
        status: "not_repo".to_string(),
        error: None,
    }
}

fn parse_branch_ab(value: &str) -> (i32, i32) {
    let mut ahead = 0;
    let mut behind = 0;
    for part in value.split_whitespace() {
        if let Some(raw) = part.strip_prefix('+') {
            ahead = raw.parse().unwrap_or(0);
        } else if let Some(raw) = part.strip_prefix('-') {
            behind = raw.parse().unwrap_or(0);
        }
    }
    (ahead, behind)
}

fn status_entry(
    path: String,
    old_path: Option<String>,
    index: char,
    worktree: char,
    kind: &str,
) -> GitStatusEntry {
    let conflicted = kind == "conflict" || index == 'U' || worktree == 'U';
    let untracked = kind == "untracked";
    let staged = !untracked && !conflicted && index != '.';
    GitStatusEntry {
        path,
        old_path,
        index_status: index.to_string(),
        worktree_status: worktree.to_string(),
        kind: kind.to_string(),
        staged,
        conflicted,
        untracked,
    }
}

fn parse_status_porcelain_v2(raw: &[u8]) -> (String, String, i32, i32, i32, Vec<GitStatusEntry>) {
    let mut head = String::new();
    let mut upstream = String::new();
    let mut ahead = 0;
    let mut behind = 0;
    let mut stash_count = 0;
    let mut entries = Vec::new();
    let records: Vec<String> = raw
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).to_string())
        .collect();
    let mut index = 0;
    while index < records.len() {
        let record = records[index].trim_end_matches('\n');
        if let Some(value) = record.strip_prefix("# branch.head ") {
            head = value.trim().to_string();
        } else if let Some(value) = record.strip_prefix("# branch.upstream ") {
            upstream = value.trim().to_string();
        } else if let Some(value) = record.strip_prefix("# branch.ab ") {
            (ahead, behind) = parse_branch_ab(value);
        } else if let Some(value) = record.strip_prefix("# stash ") {
            stash_count = value.trim().parse().unwrap_or(0);
        } else if let Some(rest) = record.strip_prefix("1 ") {
            let fields: Vec<&str> = rest.splitn(8, ' ').collect();
            if fields.len() >= 8 {
                let mut chars = fields[0].chars();
                let ix = chars.next().unwrap_or('.');
                let wt = chars.next().unwrap_or('.');
                entries.push(status_entry(
                    fields[7].to_string(),
                    None,
                    ix,
                    wt,
                    "modified",
                ));
            }
        } else if let Some(rest) = record.strip_prefix("2 ") {
            let fields: Vec<&str> = rest.splitn(9, ' ').collect();
            if fields.len() >= 9 {
                let mut chars = fields[0].chars();
                let ix = chars.next().unwrap_or('.');
                let wt = chars.next().unwrap_or('.');
                let old_path = records.get(index + 1).cloned();
                if old_path.is_some() {
                    index += 1;
                }
                entries.push(status_entry(
                    fields[8].to_string(),
                    old_path,
                    ix,
                    wt,
                    "renamed",
                ));
            }
        } else if let Some(rest) = record.strip_prefix("u ") {
            let fields: Vec<&str> = rest.splitn(10, ' ').collect();
            if fields.len() >= 10 {
                let mut chars = fields[0].chars();
                let ix = chars.next().unwrap_or('U');
                let wt = chars.next().unwrap_or('U');
                entries.push(status_entry(
                    fields[9].to_string(),
                    None,
                    ix,
                    wt,
                    "conflict",
                ));
            }
        } else if let Some(path) = record.strip_prefix("? ") {
            entries.push(status_entry(path.to_string(), None, '?', '?', "untracked"));
        }
        index += 1;
    }
    (head, upstream, ahead, behind, stash_count, entries)
}

fn dirty_counts(entries: &[GitStatusEntry]) -> GitDirtyCounts {
    let mut counts = GitDirtyCounts::default();
    for entry in entries {
        if entry.conflicted {
            counts.conflicted += 1;
        } else if entry.untracked {
            counts.untracked += 1;
        } else {
            if entry.index_status != "." {
                counts.staged += 1;
            }
            if entry.worktree_status != "." {
                counts.unstaged += 1;
            }
        }
    }
    counts
}

fn parse_git_log(raw: &str) -> Vec<GitCommitSummary> {
    let mut commits = Vec::new();
    for record in raw.split('\x1e') {
        let record = record.trim_matches('\0').trim();
        if record.is_empty() {
            continue;
        }
        let mut parts = record.splitn(2, '\0');
        let header = parts.next().unwrap_or("");
        let files_raw = parts.next().unwrap_or("");
        let fields: Vec<&str> = header.split('\x1f').collect();
        if fields.len() < 8 {
            continue;
        }
        let files = parse_name_status_records(files_raw);
        commits.push(GitCommitSummary {
            sha: fields[0].trim().to_string(),
            short_sha: fields[1].trim().to_string(),
            parents: fields[2].split_whitespace().map(str::to_string).collect(),
            refs: fields[3]
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
            author_name: fields[4].trim().to_string(),
            author_email: fields[5].trim().to_string(),
            author_date: fields[6].trim().to_string(),
            subject: fields[7].trim().to_string(),
            file_count: files.len(),
            files,
            local_only: false,
        });
    }
    commits
}

fn parse_name_status_records(raw: &str) -> Vec<GitCommitFile> {
    let mut files = Vec::new();
    let records: Vec<&str> = raw.split('\0').filter(|value| !value.is_empty()).collect();
    let mut index = 0;
    while index < records.len() {
        let status = records[index].trim();
        if status.is_empty() {
            index += 1;
            continue;
        }
        if status.starts_with('R') || status.starts_with('C') {
            let old_path = records.get(index + 1).map(|value| value.to_string());
            let path = records.get(index + 2).unwrap_or(&"").to_string();
            if !path.is_empty() {
                files.push(GitCommitFile {
                    path,
                    old_path,
                    status: status.to_string(),
                    kind: "renamed".to_string(),
                });
            }
            index += 3;
            continue;
        }
        let path = records.get(index + 1).unwrap_or(&"").to_string();
        if !path.is_empty() {
            files.push(GitCommitFile {
                path,
                old_path: None,
                status: status.to_string(),
                kind: "modified".to_string(),
            });
        }
        index += 2;
    }
    files
}

fn parse_shortstat(raw: &str) -> (usize, usize, usize) {
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    for part in raw.split(',') {
        let part = part.trim();
        if let Some(value) = part.split_whitespace().next() {
            if part.contains("file") {
                files_changed = value.parse().unwrap_or(0);
            } else if part.contains("insertion") {
                insertions = value.parse().unwrap_or(0);
            } else if part.contains("deletion") {
                deletions = value.parse().unwrap_or(0);
            }
        }
    }
    (files_changed, insertions, deletions)
}

fn split_stat_and_patch(output: &str) -> (String, String) {
    if let Some(index) = output.find("\ndiff --git ") {
        let (stat, patch) = output.split_at(index + 1);
        (stat.trim().to_string(), patch.to_string())
    } else if output.starts_with("diff --git ") {
        (String::new(), output.to_string())
    } else {
        (output.trim().to_string(), String::new())
    }
}

fn truncate_patch(value: String) -> (String, bool) {
    if value.len() <= GIT_DIFF_MAX_BYTES {
        return (value, false);
    }
    let mut end = GIT_DIFF_MAX_BYTES.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (
        format!("{}\n\n… diff truncated …\n", &value[..end]),
        true,
    )
}

fn validate_repo_relative_path(path: &str) -> Result<String> {
    let path = path.trim().replace('\\', "/");
    if path.is_empty() {
        bail!("path cannot be empty");
    }
    if path.starts_with('/') || path.contains('\0') {
        bail!("invalid path");
    }
    let mut normalized = Vec::new();
    for component in Path::new(&path).components() {
        match component {
            Component::Normal(part) => normalized.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => bail!("path escapes repository"),
        }
    }
    if normalized.is_empty() {
        bail!("path cannot be empty");
    }
    Ok(normalized.join("/"))
}

async fn build_untracked_file_patch(repo_root: &Path, path: &str) -> Result<Option<String>> {
    let clean = validate_repo_relative_path(path)?;
    let absolute = repo_root.join(&clean);
    let metadata = match tokio::fs::metadata(&absolute).await {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    if !metadata.is_file() || metadata.len() > 128 * 1024 {
        return Ok(None);
    }
    let bytes = tokio::fs::read(&absolute).await?;
    if bytes.contains(&0) {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut patch = format!(
        "diff --git a/{clean} b/{clean}\nnew file mode 100644\n--- /dev/null\n+++ b/{clean}\n"
    );
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    patch.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len().max(1)));
    if lines.is_empty() {
        patch.push_str("+\n");
    } else {
        for line in lines {
            patch.push('+');
            patch.push_str(line);
            if !line.ends_with('\n') {
                patch.push('\n');
            }
        }
    }
    Ok(Some(patch))
}

async fn resolve_state_remote(repo_root: &Path, upstream: &str) -> (String, String) {
    if let Some((remote, _)) = upstream.split_once('/') {
        if let Ok(url) = git_success(repo_root, &["remote", "get-url", remote]).await {
            return (remote.to_string(), url.stdout.trim().to_string());
        }
    }
    if let Ok(url) = git_success(repo_root, &["remote", "get-url", "origin"]).await {
        return ("origin".to_string(), url.stdout.trim().to_string());
    }
    (String::new(), String::new())
}

async fn resolve_default_base(repo_root: &Path) -> Option<String> {
    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if ref_exists(repo_root, candidate).await {
            return Some(candidate.to_string());
        }
    }
    None
}

async fn ref_exists(repo_root: &Path, reference: &str) -> bool {
    git_success(
        repo_root,
        &["rev-parse", "--verify", "--quiet", reference],
    )
    .await
    .is_ok()
}

async fn branch_exists_local(repo_root: &Path, branch: &str) -> bool {
    git_success(
        repo_root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .await
    .is_ok()
}

async fn remote_ref_exists(repo_root: &Path, branch: &str) -> bool {
    git_success(
        repo_root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/remotes/{branch}"),
        ],
    )
    .await
    .is_ok()
        || ref_exists(repo_root, branch).await
}

async fn git_remote_names(repo_root: &Path) -> Result<Vec<String>> {
    let output = git_success(repo_root, &["remote"]).await?;
    Ok(output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect())
}

async fn git_origin_exists(repo_root: &Path) -> bool {
    git_success(repo_root, &["remote", "get-url", "origin"])
        .await
        .is_ok()
}

async fn run_git(root: &Path, args: &[&str]) -> Result<()> {
    let _ = run_git_output(root, args).await?;
    Ok(())
}

async fn run_git_output(root: &Path, args: &[&str]) -> Result<GitOutput> {
    let output = git_raw(root, args).await?;
    if output.status.success() {
        return Ok(GitOutput {
            stdout: trim_bytes(&output.stdout),
            stderr: trim_bytes(&output.stderr),
        });
    }
    let message = trim_bytes(&output.stderr);
    if message.is_empty() {
        bail!("git command failed");
    }
    bail!("{message}")
}

async fn git_success(root: &Path, args: &[&str]) -> Result<GitOutput> {
    run_git_output(root, args).await
}

async fn git_raw(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .output()
        .await?;
    Ok(output)
}

fn trim_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn empty_output() -> GitOutput {
    GitOutput {
        stdout: String::new(),
        stderr: String::new(),
    }
}

fn merge_outputs(outputs: impl IntoIterator<Item = GitOutput>) -> GitOutput {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for output in outputs {
        if !output.stdout.trim().is_empty() {
            stdout.push(output.stdout);
        }
        if !output.stderr.trim().is_empty() {
            stderr.push(output.stderr);
        }
    }
    GitOutput {
        stdout: stdout.join("\n"),
        stderr: stderr.join("\n"),
    }
}

fn empty_git_diff() -> GitDiff {
    GitDiff {
        repository: false,
        branch: String::new(),
        status: String::new(),
        files: Vec::new(),
        diff: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_ab() {
        assert_eq!(parse_branch_ab("+2 -1"), (2, 1));
    }

    #[test]
    fn parses_status_records() {
        let raw = b"# branch.head main\0# branch.upstream origin/main\0# branch.ab +1 -0\01 M. N... 100644 100644 100644 111 222 333 src/main.rs\0? notes.md\0";
        let (head, upstream, ahead, behind, _, files) = parse_status_porcelain_v2(raw);
        assert_eq!(head, "main");
        assert_eq!(upstream, "origin/main");
        assert_eq!(ahead, 1);
        assert_eq!(behind, 0);
        assert_eq!(files.len(), 2);
        assert!(files[1].untracked);
    }
}
