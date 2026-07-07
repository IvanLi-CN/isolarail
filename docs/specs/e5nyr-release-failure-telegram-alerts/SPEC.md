# 发布失败 Telegram 告警接入

## 背景

仓库当前已有 `Release`、`Development Release` 与 `Docs Pages` 等交付流，但发布或部署失败时如果没有统一的 Telegram 主动告警，容易在无人值守时错过失败。

## 目标

- 为正式发布、开发发布与文档站部署失败统一接入 Telegram 告警。
- 保留一个 repo-local 的手动 smoke test 入口，便于后续 secret 轮换或链路排障。

## 非目标

- 不改动现有发布逻辑、版本策略或产物内容。
- 不新增第二套通知渠道。

## 范围

- 维护 `.github/workflows/notify-release-failure.yml` 作为发布/部署失败通知 sidecar。
- 复用 `IvanLi-CN/github-workflows/.github/workflows/release-failure-telegram.yml@main` 发送 Telegram。
- 依赖 repo secret `SHOUTRRR_URL`。

## 需求列表

### MUST

- 监听 `Release`、`Development Release` 与非 PR `Docs Pages` workflow 的失败结果。
- 手动触发 `notify-release-failure.yml` 时发送 smoke test 消息。
- 显式把 `SHOUTRRR_URL` 传给共享 reusable workflow。

### SHOULD

- 告警消息包含仓库、workflow、状态、分支、SHA、attempt、actor、run URL。

## 验收标准

- Given `Release`、`Development Release` 或非 PR `Docs Pages` 失败，When workflow 结束，Then `Notify failed release` 自动发送 Telegram 告警。
- Given 在默认分支手动触发 `notify-release-failure.yml`，When workflow 成功结束，Then Telegram 收到 smoke test 消息。

## 文档更新

- 更新 `docs/specs/README.md` 索引。

## 实现里程碑

- [x] 新增 repo-local notifier wrapper。
- [x] 配置 repo secret `SHOUTRRR_URL`。
- [x] 合并后验证 smoke test。
- [x] 将 `Docs Pages` 部署失败纳入同一通知 sidecar。
