# GitHub Actions Workflows

This project contains GitHub Actions workflows for automated building, docs deployment, release processes, and release/deploy failure notification.

## Workflow Descriptions

### 1. Code Check (`check.yml`)

**Trigger Conditions:**

- Push to `main` or `develop` branches
- Pull Requests targeting `main` or `develop` branches

**Features:**

- Code formatting check (`cargo fmt`)
- Code quality check (`cargo clippy`)
- Build main program and example programs
- Upload build artifacts

**Purpose:** Ensure code quality and successful builds

### 2. Development Release (`dev-release.yml`)

**Trigger Conditions:**

- Push to `main` branch

**Features:**

- Automatically build release version
- Generate development version number (format: `dev-YYYYMMDD-HHMMSS-commit_hash`)
- Create prerelease version
- Automatically clean up old development versions (keep latest 10)

**Purpose:** Provide testable build versions for each main branch update

### 3. Release (`release.yml`)

**Trigger Conditions:**

- Manual trigger (workflow_dispatch)

**Features:**

- Support version type selection (patch/minor/major)
- Support prerelease versions
- Automatically generate semantic version numbers
- Generate changelog
- Create official release versions

**Purpose:** Create official software release versions

### 4. Docs Pages (`docs-pages.yml`)

**Trigger Conditions:**

- Push to `main`
- Pull Requests targeting `main`
- Manual trigger (`workflow_dispatch`)

**Features:**

- Build the docs site with Bun
- Upload the GitHub Pages artifact on non-PR runs
- Deploy the docs site to the `github-pages` environment

**Purpose:** Publish the project documentation site through GitHub Pages

### 5. Notify failed release (`notify-release-failure.yml`)

**Trigger Conditions:**

- Failed `Release` or `Development Release` workflow runs
- Failed non-PR `Docs Pages` workflow runs
- Manual trigger (`workflow_dispatch`) for notifier smoke tests

**Features:**

- Sends Telegram/Shoutrrr alerts through the shared `github-workflows` reusable workflow
- Reports repository, workflow, conclusion, branch, SHA, attempt, actor, event, and run URL
- Keeps alert delivery in a sidecar workflow instead of embedding notification logic in release or deploy jobs

**Purpose:** Notify maintainers when release or docs deployment workflows fail

## Usage

### Development Workflow

1. **Daily Development:** Develop in feature branches, create PRs to the `develop` branch
2. **Code Check:** PRs will automatically trigger `check.yml` for code quality checks
3. **Integration Testing:** Continue testing after merging to the `develop` branch
4. **Release Preparation:** Merge from `develop` to the `main` branch
5. **Automatic Build:** Pushing to `main` will automatically trigger `dev-release.yml` to create development versions
6. **Official Release:** Manually trigger `release.yml` to create official versions

### Manual Release Steps

1. Go to the GitHub repository's Actions page
2. Select the "Release" workflow
3. Click "Run workflow"
4. Select version type:
   - **patch**: Bug fix version (1.0.0 → 1.0.1)
   - **minor**: Feature version (1.0.0 → 1.1.0)
   - **major**: Major version (1.0.0 → 2.0.0)
5. Choose whether it's a prerelease version
6. Click "Run workflow" to start the release

### Version Number Rules

- **Development Version:** `dev-20240101-120000-abc1234`
- **Official Version:** `v1.2.3`
- **Prerelease Version:** `v1.2.3-rc.20240101120000`

## Build Artifacts

Each workflow generates the following build artifacts:

- `isolarail` - Main firmware file (ELF format)
- `examples/*` - Example programs
- Build logs and debug information

## Important Notes

1. **Permission Requirements:** Requires `contents: write` permission to create releases
2. **Cache Optimization:** Uses Cargo cache to speed up builds
3. **Target Architecture:** Builds for `thumbv7em-none-eabihf` (STM32G431CB)
4. **Automatic Cleanup:** Development versions automatically keep the latest 10, avoiding repository bloat

## Troubleshooting

### Build Failures

- Check Rust toolchain version
- Verify dependencies are correct
- Review error messages in build logs

### Release Failures

- Confirm GitHub Token permissions
- Check for version number conflicts
- Verify build artifacts exist

### Cache Issues

- Can manually clear cache on Actions page
- Or modify `Cargo.lock` file to trigger cache update
