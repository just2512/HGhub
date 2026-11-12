# Find N2 数据源与折叠屏验收

## 数据源配置

- 固定线路、远端镜像、自定义数据源统一由「设置 → 网络 → 线路」管理。
- 自定义源只接受公网 HTTPS 根地址；添加前必须通过 Edge V1 视频列表 API 的真实请求。
- 自定义源只开放游客浏览。登录/注册及 Bearer token 只允许发送到两个固定认证 origin；关闭严格安全模式也不能扩大凭据发送范围。
- 切换数据源会清空当前会话；删除当前正在使用的数据源前必须先切换。
- 自定义源保存在应用数据目录 custom-sources.json，最多 20 条；原有 line.json 保持兼容。

## 云端自动验收

```sh
pnpm install --frozen-lockfile
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

真实上游 API 与媒体 CDN 验收（不是本地模拟）：

```sh
HGHUB_TEST_SOURCE=https://huangguoai.com cargo test --locked --manifest-path src-tauri/Cargo.toml --lib media::tests::live_play_chain -- --ignored --nocapture
```

该测试先从实时首页选择一条有可用剧集的视频（也可通过 HGHUB_TEST_VIDEO_ID / HGHUB_TEST_EPISODE 指定），请求真实 Edge V1 播放 API，获取当次播放 URL，再通过 Rust 网络层获取 m3u8、重写后的 key/分片地址并下载真实分片字节；若播放列表包含 AES-128 key，还会实际下载并核对 16 字节密钥。上游接口不可达、内容 ID/集数失效、CDN 拒绝或代理网络不可用时应如实失败；不得用本地样本替代并标记为真实通过。

## Find N2 真机验收（必须安装 ARM64 APK）

1. 外屏启动：冷启动、首页滚动、搜索、分类、设置与弹窗无裁切，底部导航不覆盖内容。
2. 内屏展开：在首页和播放页分别展开；布局应随容器宽度越过 640px 阈值切换，页面栈和选中 Tab 不重置。
3. 播放中折叠/展开：保持同一视频和集数，播放时间连续增长；不应重新请求播放地址、重建 HLS 实例或跳回 0 秒。
4. 播放链路：真实播放 m3u8；验证 AES-128 key、至少一个 TS/fMP4 分片、连续切集、拖动进度、暂停/恢复。
5. 横竖屏与全屏：进入/退出全屏后控件可触达；状态栏/导航栏安全区不遮挡按钮。
6. 后台恢复：切后台再回来，自动播放策略正确；手动暂停的视频不得自行恢复。

复现后在「设置 → 播放诊断」复制日志。重点检查 viewport.resize、web.hls.attached、network.begin/end、protocol.request、web.media.playing 和 web.media.time。折叠/展开期间若 source.load 再次出现，需单独排查组件是否被卸载。

## 验收边界

GitHub Actions 可证明前端构建、Rust 单元测试、Android ARM64 APK 构建以及 CI 网络环境下的真实上游媒体链路。它不能证明 OPPO Find N2 实机 WebView、系统栏 Insets、硬件解码和铰链切换体验；这部分必须在 PUG110 真机上执行上面的步骤。