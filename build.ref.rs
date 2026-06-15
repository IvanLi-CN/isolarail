fn main() {
    // Re-run when local metadata changes.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let env_path = std::path::Path::new(&manifest_dir).join(".env");
        println!("cargo:rerun-if-changed={}", env_path.display());
    }
    println!("cargo:rerun-if-env-changed=USB_HUB_WIFI_HOSTNAME");
    println!("cargo:rerun-if-env-changed=USB_HUB_WIFI_STATIC_IP");
    println!("cargo:rerun-if-env-changed=USB_HUB_WIFI_NETMASK");
    println!("cargo:rerun-if-env-changed=USB_HUB_WIFI_GATEWAY");
    println!("cargo:rerun-if-env-changed=USB_HUB_WIFI_DNS");
    println!("cargo:rerun-if-env-changed=USB_HUB_ALLOWED_ORIGIN");
    println!("cargo:rerun-if-env-changed=ISOHUB_RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=PROFILE");

    if is_embedded_target() {
        linker_be_nice();
    }

    inject_build_metadata();
}

fn is_embedded_target() -> bool {
    match std::env::var("TARGET") {
        Ok(target) => target.starts_with("xtensa-") || target.starts_with("riscv32"),
        Err(_) => false,
    }
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_defmt_timestamp" => {
                    eprintln!();
                    eprintln!(
                        "Note: `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("Note: Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                "esp_rtos_initialized" | "esp_rtos_yield_task" | "esp_rtos_task_create" => {
                    eprintln!();
                    eprintln!(
                        "Note: `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "Note: `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}

fn inject_build_metadata() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    register_git_rerun_inputs(&manifest_dir);

    let meta = GitBuildMetadata::collect(&manifest_dir);
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=USB_HUB_BUILD_GIT_SHA={}", meta.sha_short);
    println!(
        "cargo:rustc-env=USB_HUB_BUILD_GIT_SHA_FULL={}",
        meta.sha_full
    );
    println!("cargo:rustc-env=USB_HUB_BUILD_GIT_REF={}", meta.git_ref);
    println!("cargo:rustc-env=USB_HUB_BUILD_GIT_DIRTY={}", meta.dirty);
    println!("cargo:rustc-env=USB_HUB_BUILD_PROFILE={}", profile);
}

#[derive(Debug)]
struct GitBuildMetadata {
    sha_short: String,
    sha_full: String,
    git_ref: String,
    dirty: String,
}

impl GitBuildMetadata {
    fn collect(manifest_dir: &std::path::Path) -> Self {
        let sha_full =
            run_git(manifest_dir, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
        let sha_short = run_git(manifest_dir, &["rev-parse", "--short=12", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        let git_ref = run_git(manifest_dir, &["symbolic-ref", "--short", "HEAD"])
            .unwrap_or_else(|| "detached".to_string());
        let dirty = git_dirty_state(manifest_dir).unwrap_or_else(|| "unknown".to_string());

        Self {
            sha_short,
            sha_full,
            git_ref,
            dirty,
        }
    }
}

fn run_git(cwd: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn git_dirty_state(cwd: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(
        if output.stdout.is_empty() {
            "false"
        } else {
            "true"
        }
        .to_string(),
    )
}

fn register_git_rerun_inputs(manifest_dir: &std::path::Path) {
    let Some(git_dirs) = discover_git_dirs(manifest_dir) else {
        return;
    };

    let worktree_git_dir = git_dirs.worktree_git_dir;
    let common_git_dir = git_dirs.common_git_dir;

    println!(
        "cargo:rerun-if-changed={}",
        worktree_git_dir.join("HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        worktree_git_dir.join("index").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        common_git_dir.join("packed-refs").display()
    );

    let head_path = worktree_git_dir.join("HEAD");
    let Ok(head_contents) = std::fs::read_to_string(&head_path) else {
        return;
    };
    let Some(reference) = head_contents.strip_prefix("ref: ") else {
        return;
    };
    let reference = reference.trim();
    if reference.is_empty() {
        return;
    }

    println!(
        "cargo:rerun-if-changed={}",
        common_git_dir.join(reference).display()
    );
}

struct GitDirs {
    worktree_git_dir: std::path::PathBuf,
    common_git_dir: std::path::PathBuf,
}

fn discover_git_dirs(manifest_dir: &std::path::Path) -> Option<GitDirs> {
    let dot_git = manifest_dir.join(".git");
    if dot_git.is_dir() {
        return Some(GitDirs {
            worktree_git_dir: dot_git.clone(),
            common_git_dir: dot_git,
        });
    }

    let git_file = std::fs::read_to_string(&dot_git).ok()?;
    let git_dir_str = git_file.trim().strip_prefix("gitdir:")?.trim();
    let worktree_git_dir = resolve_git_path(dot_git.parent()?, git_dir_str);

    let common_git_dir = std::fs::read_to_string(worktree_git_dir.join("commondir"))
        .ok()
        .map(|value| resolve_git_path(&worktree_git_dir, value.trim()))
        .unwrap_or_else(|| worktree_git_dir.clone());

    Some(GitDirs {
        worktree_git_dir,
        common_git_dir,
    })
}

fn resolve_git_path(base: &std::path::Path, raw: &str) -> std::path::PathBuf {
    let candidate = std::path::PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    }
}
