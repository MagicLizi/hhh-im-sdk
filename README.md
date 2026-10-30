# hhh-im-sdk

给 [HHH](https://github.com/MagicLizi) 写 IM 插件用的协议包与工具。HHH 的 IM 层只做统一的界面和一个统一的插座
（HHH-IM-Protocol）：每种 IM 都是插在这个插座上的插件，负责和自己的平台说话、把平台的格式翻成统一格式、声明自己能做什么
和要用户填什么；聊天记录、凭据、配置都交给 HHH。

本仓库从 HHH 仓库按版本同步过来（HHH 仓库是唯一正本），不要在这里直接改协议。

## 里面有什么

| 内容 | 说明 |
|---|---|
| `hhh-im-protocol/` | 协议的 Rust 包：线上类型（含格式正文与卡片的 `content`）、JSON-RPC 编解码、写插件程序用的运行时（feature `runtime`）、子进程 stdio 传输（feature `transport`）、安装包 `.hhhim` 的打包与解包（feature `package`） |
| `hhh-im-net/` | 联网模块：插件向平台发 HTTP、直接开 TCP 都用它。电脑上就是 reqwest，另加防掐断的握手设置；iPhone / iPad 上是接口同名的一套实现，插件代码不分平台 |
| `hhh-im-check` | 自检工具：拿你的插件把协议用例跑一遍 |
| `hhh-im-pack` | 打包工具：把插件文件夹打成 `.hhhim`、算 sha256、写版本索引；`--publish` 发到 GitHub Release 或 GitLab |
| `docs/protocol-v1.md` | 协议规范 |
| `docs/release.md` | 发版本规范：标签、Release、版本索引、包放在哪 |

## 用法

插件里依赖协议包与联网模块（按标签取一个固定版本，两个包同一个标签）：

```toml
[dependencies]
hhh-im-protocol = { git = "https://github.com/MagicLizi/hhh-im-sdk", tag = "v0.21.0", features = ["runtime"] }
hhh-im-net = { git = "https://github.com/MagicLizi/hhh-im-sdk", tag = "v0.21.0" }
```

联网一律用 `hhh-im-net`，不要直接依赖 reqwest：电脑上它就是 reqwest（`hhh_im_net::Client` 等原样导出），建客户端从
`hhh_im_net::client_builder()` 开始（装好了防掐断的握手）；编 iPhone 版时同一份代码换成接口同名的实现，哪些用法没有见
`hhh-im-net` 的文档。出错打日志用 `hhh_im_net::network_cause(error)`：带全部底层原因、不带请求地址。

运行时（`hhh_im_protocol::runtime`）管每个插件都要的、和平台无关的那一半：读写标准输入输出、宿主的请求各开一个任务并发处理、
`$/cancel`、向宿主发请求并等回复（`credential/save`、`events/push`）、日志转 `log/write`、资源分段读、上传攒齐与核对、
向平台发了几次请求。插件实现 `Plugin`（收到某个方法怎么办），交给 `serve_on` / `serve_stdio` / `serve_stdio_on`；模板仓库就是这样写的。
声明 `readRequestGating` 后，在每条平台读取出口等待 `metering::read_request().await?`，宿主放行后再发 HTTP；运行时自动确认支持回执的许可，避免通信恢复时积攒许可集中生效。旧宿主没有给额度凭据时仍按结果计数。详见协议 §8。
编成 `wasm32-wasip2` 给 iPhone / iPad 用时，`serve_stdio*` 在 WASI 上自己接好标准输入输出（见 `docs/release.md` §2）。

账号级操作可声明 shortcut:true，让宿主在会话区显示快捷入口；操作完成可返回 openConversationId，支持宿主核对并打开结果会话（旧宿主可忽略）。详见协议 §6.7。

发送格式正文可声明 capabilities.richSend（formats 与 edit），用 Outgoing::Rich 复用收消息的 RichNode；未声明时宿主继续交纯文字。
插件将节点翻成平台的原生格式，能带格式编辑时也返回对应的 editDraft。详见协议 §7.8。

装两个工具：

```sh
cargo install --git https://github.com/MagicLizi/hhh-im-sdk --tag v0.21.0 hhh-im-protocol --features transport,package
```

- 自检：`hhh-im-check <插件文件夹>/hhh-im-provider.json`
- 阅读能力：`receipts`、`markRead`、`readSync` 分开声明，见协议 §8.1；`--reading-chat <会话编号>` 指定只读检查范围，写回另需 `--allow-mark-read <确切消息编号>`，不能与 `--read-only` 同用。
- 已登录的真实发送自检：加 `--allow-send <已获授权的测试会话编号>`；按 richSend 声明检查格式正文，未提供发送授权时跳过，发送结果不确定时不自动重发
- 打包：`hhh-im-pack <插件文件夹> --target windows-x64`（不写 `--target` 用这台电脑的目标）
- 发布：`hhh-im-pack <插件文件夹> --notes notes.md --publish`，令牌放环境变量 `HHH_IM_PUBLISH_TOKEN`（GitHub 也认 `GITHUB_TOKEN`，GitLab 也认 `GITLAB_TOKEN`），不写进任何文件
- GitLab 发布会同时更新包仓库的最新版本索引；旧版本可用 `hhh-im-pack <插件文件夹> --publish --latest-only` 只补索引，清单版本必须已发布

私有插件的拥有者可在支持分享码的 HHH 中点「管理 → 分享…」，朋友粘贴分享码安装和更新。GitHub 的清单 `source` 应指向单独的发布仓库，只放安装包与版本索引；GitLab 分享使用只读包仓库的部署令牌。发布令牌留在发布者电脑，不能当分享码发给朋友。详见 [发版本规范](docs/release.md)。

从零起一个插件，用模板仓库 [hhh-im-plugin-template](https://github.com/MagicLizi/hhh-im-plugin-template)：清单、标志、插件骨架、打包脚本、
给 agent 的规范都在里面。

## 许可证

[MIT](LICENSE)
