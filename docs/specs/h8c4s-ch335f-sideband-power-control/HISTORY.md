# CH335F Sideband 电源控制历史

## 2026-06-13

- 将本规格中的控制面命名继续对齐到当前项目口径：不再把 `host` 侧表述混入本项目软件包命名，也不把与当前四路 Hub 无关的旧双口 `USB-C` 控制语义写成当前规格目标。
- 明确本规格只约束固件内的 CH335F sideband 门控与上游侧联动事实，不承担 `isohub` / `isohub-devd` owner-facing 控制面产品化。
- 真机验证发现当前板卡可报告 `isolated_usb_fault=true`，但 owner 仍需要显式 POWER ON
  端口；固件将 sideband helper 离线路径调整为 degraded manual mode，保留 fault telemetry
  同时允许手动输出。

## 关键演进

- 初始实现将 `TCA6408A@0x20` 的 P0/P2/P4/P6 作为低有效 `PWREN#` 输入，P1/P3/P5/P7 作为低有效 `OVCUR#` 注入。
- 实物验证发现没有连接上游主机时，CH335F `PWREN#` 默认导致四路输出关闭；产品需求要求未连接电脑时仍能独立供电输出。
- 固件引入 `GPIO21/V1OK` 判断 ISOUSB211 side-1 power 状态：`V1OK=low` 使用 standalone 模式，`V1OK=high` 使用 upstream-managed 模式。
- 上游侧 `uhubctl` 验证确认 CH335F 端口开关可以驱动对应固件输出门控，但当前硬件存在 `PWREN1#` / `PWREN2#` 连接错误；硬件缺陷已登记为 GitHub issue #18。

## References

- `docs/ch335f_tca6408a_appnote.md`
- `docs/hardware/mainboard_netlist.enet.enet`
- `docs/software_design.md`
