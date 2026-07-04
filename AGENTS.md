# AGENTS.md

本文件为本仓库内“智能代理/代码助手”的工作约定与执行指引。其作用域为仓库根目录起的整个目录树；如未来出现子目录下更深层的 AGENTS.md，则更深层文件优先生效。

## 快速要点

- 语言与目标：Rust，目标 `xtensa-esp32s3-none-elf`（ESP32‑S3）。
- 入口与结构：`src/main.rs` 为示例程序，使用 `esp-hal` + `embassy` 异步框架。
- 运行器：由 `.cargo/config.toml` 指定 `runner = "bash tools/isohub-runner"`，复用本仓 Local USB CLI/devd 烧录路径。
- 打印与日志：使用 defmt + `esp-println`；默认 `DEFMT_LOG=info`，可通过该环境变量调整日志级别。
- 提交规范：必须使用 Conventional Commits；不得绕过校验；不得擅自 push。
- 钩子与工具：`lefthook.yml` 使用 `cargo +esp fmt/clippy` 与 `bunx` 的 commitlint/markdownlint。
- 文档示意图：UI 效果图为像素级 160×50（W×H）资源（像素网格为 50×160，行×列），必须以每像素 1×1 矩形的彩色 SVG 表达（RGB565 颜色近似）。仅维护两张示意图（正常态与混合状态），统一存放于 `docs/assets`，并在 `docs/dashboard_spec.md` 中引用。

### 代理执行准则

- 在请求主人验证或合入前，代理必须先在本地完成一次可复现的构建（先 `cargo check`，然后至少 `cargo build --release` 成功）。
- 仅在本地构建/自检通过后，才向主人提交验证请求或评审说明；未通过时应先自行排查并给出修复方案。

## 环境与工具链

- 推荐使用 `espup` 安装 ESP Rust 工具链：
  1) 安装 `espup` 后执行 `espup install`；
  2) 在 shell 中 `source ~/export-esp.sh` 以导出 `esp` 工具链与 Xtensa 目标；
  3) 安装 `espflash`：`cargo install espflash`（由 `isohub-devd` 后端调用）；
  4) 安装 `just`：`cargo install just` 或使用系统包管理器。
- 本仓库的 pre-commit 钩子使用 `cargo +esp`，要求本机存在名为 `esp` 的 Rust 工具链（`rustup toolchain list` 可查看）。
- 若运行 markdown/commit 校验，需要本机安装 `bun`（`bunx` 用于运行 `commitlint` 与 `markdownlint-cli2`）。

## 构建与烧录

- 构建：`cargo build` 或 `cargo build --release`。
- 推荐烧录与串口监视：`just flash-monitor`。首次使用先执行 `just ports`，再用 `PORT=/dev/cu.xxx just identify` 写入 `.esp32-port`。
- `cargo run --release` 会通过 `tools/isohub-runner` 复用同一 Local USB 身份校验和烧录路径。
- 旧烧录入口已完全退役：不得使用 Makefile、裸 `espflash flash --monitor` 或 `mcu-agentd` 作为本仓烧录路径。

## 代码与修改约定

- 风格：保持改动最小、聚焦问题本身；不做无关重构；遵循现有风格与依赖版本。
- 格式与静态检查：
  - 使用 `rustfmt` 与 `clippy`；本地可通过 `lefthook` 自动执行 `cargo +esp fmt` 与 `cargo +esp clippy`。
  - Markdown 使用 `markdownlint-cli2`，规则由 `.markdownlint-cli2.yaml` 提供。
- 依赖与文件：
  - 非必要不新增依赖；不更改许可证与版权头；不做大范围重命名与移动。
  - 新增模块优先置于 `src/` 并保持结构简单清晰。
- 文档：如修改构建/运行方式，请同步更新 `README.md` 或相关文档。
- 文档登记：
  - 硬件连接：`docs/hardware_connection_overview.md`（涵盖电源输入、PCA9545A 下行通道与四路 USB 供电模块、TCA6408A 五向开关、INA226/TPS2490 监测与保护等）。若地址/连线变更，需同步更新本文件与 `README.md` 索引。
  - 软件设计总文档：`docs/software_design.md`（统一日志风格、全局目标状态、调度建议；收录“基础电源输入子系统”等模块的软件规范）。

永不修改 vbus_ratio（禁写 RATIO 0x08 bit0）。

## 提交与分支

- 提交信息：必须遵循 Conventional Commits，例如：
  - `feat: xxx`、`fix: yyy`、`docs: zzz`、`refactor: ...`、`chore: ...` 等。
- 钩子与校验：
  - 不得使用 `--no-verify` 绕过校验。
  - 使用 `--amend` 前须确认当前提交历史符合预期，避免丢失他人工作。
- 不要擅自 `git push`；如需提交或推送，需明确任务指示。

## CI 注意事项（重要）

- 现有 GitHub Actions（如 `.github/workflows/check.yml`、`release.yml`、`dev-release.yml`）仍指向 `thumbv7em-none-eabihf` 与二进制名 `iso-usb-hub`（STM32 相关）。
- 当前工程示例为 ESP32‑S3（Xtensa）；如需对齐 CI 到 ESP32‑S3，请在任务中明确授权后再执行：包括更新 target、构建命令、产物名等。
- 未经授权不得擅自修改 CI 工作流。

## 测试与验证

- 目前仓库未提供 Rust 单元/集成测试框架；如确需新增，需最小化引入并与现有结构保持一致。
- 运行时验证以 `just flash-monitor` 或 `cargo run --release`（通过 `isohub` / `isohub-devd`）串口日志为主；请在 PR/变更说明中给出期望输出或观测结果。

## 常见问题

- `cargo +esp` 无法运行：确认已通过 `espup install` 安装 `esp` 工具链并 `source ~/export-esp.sh`。
- `isohub` 未找到设备：先运行 `just ports`，再用 `PORT=/dev/cu.xxx just identify` 明确端口；全新硬件或下载模式需经 `PORT=/dev/cu.xxx just flash-first-time` 的 typed confirmation。
- Markdown/提交校验失败：确认本机安装 `bun`，并以 `bunx` 调用对应工具；根据报错修复格式或提交信息。

---

请所有自动化/智能代理严格遵守本文件约定执行任务；如遇规则冲突，以更深层目录的 AGENTS.md 或直接任务指令为准。
