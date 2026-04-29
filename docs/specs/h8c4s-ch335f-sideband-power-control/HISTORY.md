# CH335F Sideband 电源控制历史

## 关键演进

- 初始实现将 `TCA6408A@0x20` 的 P0/P2/P4/P6 作为低有效 `PWREN#` 输入，P1/P3/P5/P7 作为低有效 `OVCUR#` 注入。
- 实物验证发现没有连接上游主机时，CH335F `PWREN#` 默认导致四路输出关闭；产品需求要求未连接电脑时仍能独立供电输出。
- 固件引入 `GPIO21/V1OK` 判断 ISOUSB211 side-1 power 状态：`V1OK=low` 使用 standalone 模式，`V1OK=high` 使用 host-managed 模式。
- host-side `uhubctl` 验证确认 CH335F 端口开关可以驱动对应固件输出门控，但当前硬件存在 `PWREN1#` / `PWREN2#` 连接错误；硬件缺陷已登记为 GitHub issue #18。

## References

- `docs/ch335f_tca6408a_appnote.md`
- `docs/hardware/mainboard_netlist.enet.enet`
- `docs/software_design.md`
