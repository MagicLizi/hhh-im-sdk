# HHH-IM-Protocol v1 规范（草案）

> 状态：draft，随 [contract-v1](contract-v1.md) 一起确认。日期：2026-10-04。
> 本文规定宿主（HHH）与外部 IM 程序之间交换什么。程序内部怎么写，见[通用实现范式](provider-guide-v1.md)。

接入范围以当前宿主实现的声明和方法为准。新平台使用这些能力，只需提供插件程序、清单与安装包，不登记到 Friend 或 hhh-im；超出范围的能力须先扩展通用协议和宿主实现。SDK 的包版本与线上协议主版本是两件事，不能仅凭 SDK 已提供某个类型就认定所有宿主已支持它。

## 0. 术语

- **宿主**：HHH。具体由 hhh-im 的“外部程序适配器”实现，Friend 负责装配。
- **IM 程序**（简称程序）：实现本协议的一个独立可执行程序，用什么语言写都行。它在 HHH 里的位置和飞书、Slack 适配器相同，只做三件事：**拿数据、做动作、报变化**。
- **连接**：HHH 里的一个 IM 账号。一个程序进程只服务一个连接。

## 1. 原则（小版本里不改）

1. **数据和配置全部在 HHH。** 宿主用 HHH 现有的本地库、文件库和系统凭据库保存所有东西：聊天记录、话题、表情、已读未读、发送记录、人名册、会话资料、文件、凭据、同步位置和程序设置。程序不落盘任何业务数据，只能用内存缓存，以及宿主给的临时目录（宿主随时可能清空它）。
2. **一个进程一个账号。** 宿主给每个连接启动一个程序进程。协议里的编号都限定在这个账号里，所以不带账号编号或连接编号。
3. **按能力声明办事。** 握手时程序交出平台说明和能力清单。宿主只调用声明过的方法，界面只显示声明过的操作。没有声明发送能力的连接是只读的。
4. **宿主校验是为了保数据、防 bug，不是安全隔离。** 大小、数量、频率、超时和编号格式都由宿主检查；程序声明的上限只能比宿主的更严，不能更松；程序给的网址界面一律不直接打开。程序以用户自己的权限运行，真正的信任边界在“安装”这一步——用户看清楚程序路径、来源和能力后才启用。
5. **推送至少送达一次。** 程序推来的变化，宿主写进本地库之后才确认；没得到确认的，程序要重发。重复到达的内容按编号去重。
6. **平台私有的东西留在程序里。** 私有编码、私有格式、加密和签名，都由程序转换成本协议的通用形式。宿主看不到平台的原始数据。程序可以随消息附上**去凭据化**的原始数据，供以后重新翻译（§7.4、§10.1），这份数据里不能含令牌、密钥或别的凭据。
7. **向前兼容。** 不认识的字段，双方都忽略；不认识的内容块，显示为“暂不支持”；新功能只以可选能力的方式加入。

## 2. 程序包

每个程序带一个清单文件 `hhh-im-provider.json`，放在程序文件夹的最上层。用户在 HHH「设置 → IM → 添加 IM」里选安装包 `.hhhim`、填仓库地址或选程序文件夹（开发用）；HHH 读清单、试启动一次，通过后把程序复制（安装包是解开）进自己的 IM 插件目录再运行，原文件之后可以删或挪。更新来自清单 `source` 写的仓库（点“更新”），也可以「从文件更新」选新版本的安装包或文件夹（编号必须相同）。

安装包、版本索引与发版本的规矩见[发版本规范](release.md)（公开仓库 `hhh-im-sdk` 的 `docs/release.md` 是它的副本），要点（契约 v10 §4.1–§4.3）：安装包就是 zip，最上层是清单；一个包只给一种电脑（`windows-x64`、`macos-arm64`、`macos-x64`、`linux-x64`、`linux-arm64`）；`command` 不写扩展名时，Windows 包里指向 `<command>.exe`，宿主在 Windows 上找不到 `command` 时也试 `<command>.exe`。

```json
{
  "manifestVersion": 1,
  "id": "wechat-pad",
  "name": "微信",
  "version": "0.3.0",
  "command": "bin/wechat-provider.exe",
  "args": ["--hhh"],
  "os": ["windows"],
  "logo": "logo.svg",
  "publisher": "Lizi",
  "homepage": "https://example.com",
  "imType": "wechat"
}
```

| 字段 | 规则 |
|---|---|
| `manifestVersion` | 清单格式的版本，现在是 1 |
| `id` | 程序编号：以小写字母开头，只含 `a-z0-9-`，长 2–32 个字符。普通插件的平台编号为 `ext-<id>`；带官方签名的保留编号按官方目录的平台编号映射，旧平台沿用原编号，以连接已有历史与关注。同一个编号只能装一个程序；回调路径可写相对路径（§6.1a），程序不必推断宿主编号。宿主不再提供内置平台实现，旧账号交接见附录 D |
| `name` | 给用户看的平台名，1–20 个字符 |
| `version` | 程序版本，semver 写法 |
| `command` / `args` | 启动命令。`command` 按清单所在目录解析相对路径，也可以是 PATH 里能找到的程序（如 `node`、`python`）；`args` 原样传入 |
| `os` | 支持的系统：`windows` / `macos` / `linux`。当前系统不在列表里时不能添加 |
| `logo` | 可选。SVG、PNG、JPG 或 WebP，不超过 256 KiB，路径相对清单所在的文件夹、不能走出这个文件夹。没有 logo 时，宿主用名字的第一个字画一个 |
| `logoStyle` | 可选（v12）。`icon`（默认）：logo 是一张完整的应用图标，宿主原样画；`mark`：logo 是透明底的标志图形，宿主垫自己的底色与外框 |
| `logoInk` | 可选（v12）。`{x, y, width, height}`：标志图形在画布里实际占的那一块（按 logo 自己的坐标），宿主据此居中、缩放。四个数要是有限数，宽、高大于 0 |
| `publisher` / `homepage` | 可选。添加时展示给用户 |
| `imType` | 可选（v5）。IM 类型，规则同 `id`；相同值表示同一种，宿主里同一种同一时间只用一个。不写时自成一类、不和其他 IM 互斥。类型只认这个字段，不从插件编号推断 |
| `source` | 可选（v10）。更新来源：`{"type":"github","repo":"<owner>/<name>"}`、`{"type":"gitlab","host":"https://<站点>","project":"<group>/<name>"}`、`{"type":"url","index":"https://…/hhhim-release.json"}` 之一。不写就没有更新来源。写了时 `version` 必须是 `主.次.修订` 三段数字。宿主只接受同一来源、同一编号的更新 |
| `minHost` | 可选（v10）。需要的最低 HHH 版本（`主.次.修订`）；HHH 更旧时不给装、不给更新 |
| `target` | 可选（v10）。这个包给哪种电脑，打包工具按 `--target` 写进包里的清单，不用手写。从安装包装时必须有且是本机；文件夹装时可以没有，照旧按 `os` 判断 |

清单里只放启动前就必须知道的信息。平台说明和能力在握手时给出（§5），可以随程序版本变化。

## 3. 传输

- 消息格式是 **JSON-RPC 2.0**。宿主发给程序的消息走程序的标准输入，程序发给宿主的走标准输出。编码 UTF-8，**一行一条消息**：以 `\n` 结尾，消息内部不能出现原始换行。
- 双方都可以发请求和通知。程序发给宿主的请求限于本文定义的方法，包括可选宿主显示名（§4.5）、保存凭据（§6.3）、读取额度（§8）与推送（§11.1）。
- **标准输出只能输出协议消息。** 宿主每读到一行不合法的 JSON-RPC，记一次协议错误；同一个进程连续 3 次，宿主就重启它。
- **标准错误输出是日志**：每行一条。宿主把每行截到 4 KiB，标上程序编号，写进 HHH 日志。日志里不要写凭据和消息正文。
- **单条消息最大 4 MiB**。超过的消息宿主直接丢掉，按协议错误处理。大文件要分段传（§9）。
- 宿主启动程序时，设置环境变量 `HHH_IM_PROTOCOL=1`，工作目录是一个空的临时目录。凭据不通过环境变量传。
- 第一版只有这一种传输方式。方法和类型都不依赖传输，以后可以再加本机 WebSocket 或远程连接。
- **结构化日志**：除了标准错误输出，程序还可以发 `log/write` 通知（§11.5），把一条带级别和字段的日志交给宿主，宿主并进自己的日志系统。两者都不能写凭据或消息正文。

## 4. 生命周期

```
启动进程 → initialize → session/open ─┬→ 在线：读取 / 动作 / 推送
                                      └→ 需要登录：login/start … credential/save → login/finished → 在线
在线时程序发 session/expired（授权失效）→ 宿主提示重新登录
任何时候 → shutdown → 宿主关闭标准输入 → 程序 5 秒内退出（超时宿主就结束进程）
```

### 4.1 initialize（宿主→程序，必需）

参数：

```json
{
  "protocolVersions": [1],
  "host": { "name": "HHH", "version": "0.9.0" },
  "locale": "zh-CN",
  "tempDir": "<宿主给的临时目录，可随时被清空>",
  "network": { "proxy": null }
}
```

结果：

```json
{
  "protocolVersion": 1,
  "provider": { "id": "wechat-pad", "name": "微信", "version": "0.3.0" },
  "platform": { "…": "平台说明，见 §5.1" },
  "capabilities": { "…": "能力，见 §5.2" },
  "settings": [ "…程序设置项，见 §6.4" ],
  "loginMethods": [ "…可选，登录方式，见 §6.1a" ]
}
```

- `protocolVersion` 必须是宿主列出的版本之一，否则宿主拒绝连接，并提示“程序版本与 HHH 不兼容”。
- `provider.id` 必须和清单里的一致。
- 结果可声明 `readRequestGating: true`，表示每条平台读取都支持发送前向宿主申请额度（§8）；默认关闭，旧程序和旧宿主仍按 `platformRequests` 结算。
- 宿主保存最近一次收到的平台说明。没连上程序时，宿主按这份说明只读打开本地库。
- `network.proxy` 是 HHH 的出站代理设置。程序访问平台时应当照这个设置走，这样用户在 HHH 里设的网络规则对程序也生效。
- 超时 10 秒。

### 4.2 session/open（宿主→程序，必需）

用宿主保存的凭据、设置和同步位置恢复账号。

参数：

```json
{ "credential": "<不透明字符串，或 null>", "settings": { "<键>": "<值>" }, "syncState": "<不透明字符串，或 null>",
  "credentialReadOnly": true, "legacy": { "format": "hhh-builtin-v1", "credential": "…", "settings": {} } }
```

- `credentialReadOnly`（可选，v12）：这份凭据是借来的、只能用（例如隔离测试环境借用户本人的）。为真时程序不得换新凭据、不得调 `credential/save`（宿主一律回错误），到期就发 `session/expired`。不带即为假。
- `legacy`（可选，v12）：旧内置平台留下的授权与设置，程序还没有自己的凭据且交接未完成时可再次收到，格式见附录 D。

结果是下面两种之一：

- `{ "state": "online", "account": Account, "adoptedSettings"?: { "parameters"?: { "<设置项编号>": "<值>" }, "login"?: { "method", "values" } } }`
- `{ "state": "needsLogin", "reason": "noCredential|expired|revoked|credentialInvalid", "message": "<给用户看的说明>" }`

`adoptedSettings`（可选，v12）：请宿主写入程序参数（§6.4）与当前登录方式（§6.1a）。接过 `legacy` 的程序应把这些选择留在自己的不透明凭据中；此后普通 `session/open` 成功时仍可返回，便于恢复“凭据已保存、设置尚未保存就中断”的交接。宿主只在在线、凭据已持久保存、参数与登录选择都写入后才删除旧栏；任一步失败保留原件并重试。只读借用时不删除来源凭据或旧栏，也不保存交接选择。

规则：

- `account.id` 和宿主上次核验的账号不同时，宿主按现有规则处理（hhh-im 的 `AccountChanged`）：停止读写旧账号的库，提示用户换了账号。
- 超时 30 秒。

### 4.3 shutdown（宿主→程序）

没有参数。程序停掉所有平台请求和推送，回复空结果，然后等标准输入关闭后退出。

### 4.4 心跳与重启

- 宿主每 30 秒发一次 `ping`（没有参数，结果为空）。连续两次在 10 秒内没有回复，就当作程序卡死。
- 程序退出或卡死时，宿主按 1、2、4……最长 60 秒的间隔退避重启。10 分钟内重启 5 次还不行，宿主就停下来，在界面上把这个连接标成“程序出错”，并附上最后几行日志。
- 每次重启都从 `initialize` 开始，再用保存的凭据和同步位置接着上次的进度。

### 4.5 host/userName（程序→宿主，可选）

`initialize` 完成后，程序可以按需发送 `host/userName`，参数为空对象 `{}`。结果为 `{ "name": "用户显示名" }`，无可用名字时为 `{}`。

- `name` 是当前业务用户自己已保存的显示名；在 Friend 中来自个人资料，游客同样适用。它不是 IM 昵称、操作系统用户名或登录令牌。
- 仅返回名字，不附身份编号、邮箱、头像、SOUL 或凭据。自定义与官方插件均可请求；不赋予“用宿主账号登录”的权限。
- 每次请求读取当前值。连接撤销或所属身份已切换时返回 `{}`，不交出另一个身份的资料。程序不应持久化副本。
- 旧宿主可能回复 `{}` 或方法不存在；程序必须有无名字的默认行为，并为可选请求设置短超时，不以此阻止登录与使用。不要在处理 `initialize` 时等待此请求，以免握手互相等待。
- 各 IM 如何使用这个名字由程序决定，宿主不规定平台设备名格式。

## 5. 平台说明与能力

### 5.1 平台说明（`platform`）

宿主根据平台说明来限速、分页、显示界面，并计算每条消息允许哪些操作。它对应 hhh-im 的 `Platform` / `PlatformInfo`。

| 字段 | 类型 | 必需 | 说明 |
|---|---|---|---|
| `avatarShape` | `circle` \| `rounded` | 是 | 头像在平台自己的客户端里是什么形状 |
| `avatarIdentity` | 对象 | 否 | v14。`{keyPattern, variantPattern?}`：头像键里的身份。平台给同一个头像的地址会变（如换了 CDN 主机）、程序又拿地址当键时写它：`keyPattern` 的第一个括号是头像编号（对不上时整个键就是身份），`variantPattern` 的第一个括号是尺寸这类变体（不写或对不上时没有变体）。身份相同宿主就当没变：资料不算变了、头像不重下。两个都是 Rust `regex-lite` 的正则（不支持环视与反向引用） |
| `chatLink` | 字符串 | 否 | 在平台客户端里打开会话的链接模板，`{conversation}` 会换成 URL 编码后的会话编号。不给就不显示“在 xx 里查看” |
| `imageNamePrefix` | 字符串 | 否 | 用户下载图片时文件名的开头，默认用平台名 |
| `conversationNouns` | `{group, direct, channel, unknown}` | 否 | 各类会话怎么称呼，默认依次是“群 / 私聊 / 频道 / 会话” |
| `history` | `server` \| `none` | 是 | `server`：消息随时可以向平台重新读取（飞书、Slack 这类）。`none`：平台只推一次，不能翻历史（微信这类）。见 §11.4 |
| `pageMax` | 正整数 | `history=server` 时必需 | 按会话读消息时，一页最多几条。协议不设上限（v12），按平台自己的上限写 |
| `reads` | `{perSecond, perMinute}` | 否 | 读取请求的合计上限。不写按宿主默认（每秒 5 次、每分钟 120 次，v12 起不再必填） |
| `downloads` | `{perSecond, perMinute}` | 否 | 下载消息资源的上限，平台不限就省略 |
| `fileSegmentBytes` | 正整数 | 否 | 文件按多大一段读，支持续传。不给就整份读，断了从头重下。协议不设上下限（v12） |
| `limits` | `{imageUploadMaxBytes, fileUploadMaxBytes, attachmentNameMaxBytes}` | 能发图片或文件时必需 | 发送附件的上限 |
| `readDefaults` | 读取间隔（秒） | 否 | 默认读取间隔，字段同 hhh-im 的 `ReadIntervals`，另可写 `discover`（会话发现间隔，v12）。超出宿主允许范围的值会被拒绝 |
| `userAdjustable` | 字符串数组 | 否 | v12。开放给用户在「参数配置 → 读取与下载」里改的项：`readIntervals`（读取频率一组）、`pageMax`、`readsPerSecond`、`readsPerMinute`、`downloadsPerSecond`、`downloadsPerMinute`、`fileSegmentBytes`。叫法、单位、控件由宿主统一，默认值取本平台说明里写的；改了立即生效。不认识的取值宿主忽略。一项都不写就没有这一组 |
| `discovery` | `{fastestSecs, pages, coldPages}` | 否 | v12。会话发现：宿主定时找一段时间里有人发过消息、不在列表里的会话，加进会话列表。声明了能力 `conversations.recentActivity` 时问程序（`conversations/recentActivity`，§8）；没有它但同时声明 `conversations.list` 与 `messages.list` 时，宿主自己逐会话限频探测（`conversations/list` 列会话，每个不在列表里的再 `messages/list` 读起点之后最新几条，请求按 `reads` 额度排在后台）；都没有不发现。`fastestSecs` 是最快多久找一次，`pages` / `coldPages` 是平常与刚启动时一次最多翻几页（账号级查询翻结果页，逐会话探测翻会话列表页） |
| `editsByContent` | 字符串数组 | 否 | v12。平台不给“改过”标记时，宿主按内容认编辑：这里列出的内容块种类（`text`、`mention`、`rich`）真变了、时间往后才算编辑过。不写就只认 `editedAtMs` |
| `retranslateRules` | `[{name, formats?}]` | 否 | v12。程序现在的翻译规则名，按顺序（§10.1）。`formats` 给了就只把 `raw.format` 在其中的旧消息交给这条规则。至多 32 条，名字不能重复 |
| `contentRepairs` | 字符串数组 | 否 | 一次性本地修正规则名，按依赖顺序执行（§10.2），须同时声明 `messageMaintenance` |
| `contentRereads` | 字符串数组 | 否 | 视野内旧消息的重读判定规则名（§10.2），须同时声明 `messageMaintenance` |
| `threads` | 对象或 null | 否 | 平台有话题时给：`{activityOnRoot, readsAfter, broadcast: {native, attachments} 或 null}`，含义同 hhh-im `Platform` 里的同名项 |
| `deletedMessagesVanish` | 布尔 | 否 | 撤回的消息直接从历史里消失（而不是留下“已撤回”标记）时为真。默认假 |
| `reactionsInReads` | 布尔 | 否 | 读消息时会随消息带回完整的表情。默认假 |
| `emojiPerAccount` | 布尔 | 否 | 不同账号的表情目录不一样。默认假 |
| `actions` | 见下文 | 是 | 每条消息允许哪些操作 |
| `emoji` | `{namePattern, unlisted: {placeholderPattern, placeholderLabel, guess?, guessPattern?}, textPattern?, whiteBackground?, textTemplate?}` | 有表情能力时必需 | 表情编号的规则（正则，JavaScript 语法；v14 起宿主照它核对点表情的编号），以及目录里没有的表情怎么显示（v14 起宿主照 `placeholderPattern`、`placeholderLabel` 画占位）。v12：`unlisted.guess` 为真时，目录外的表情类型也问 `emoji/image` 要图（§9.1），v14：`guessPattern` 给了就只拿合它的类型去问（猜不出的如飞书企业表情的纯数字编号不必每次问）；`textPattern` 是正文里表情的写法（JavaScript 正则，第一个括号是表情名，例如飞书的 `\[([^\[\]\r\n]{1,12})\]`），宿主把正文里这样写、目录里有这个名字的换成表情图，不写就不换；`whiteBackground` 为真时表情图是白底的，宿主显示前抠成透明。v31：`textTemplate` 是表情写进文字的写法，恰好一个占位 `{name}`（目录里的名字）或 `{type}`（表情编号），不含换行与控制字符，去掉占位后至多 16 个字符（如飞书、微信 `[{name}]`，Slack `:{type}:`）；宿主据此把输入框表情面板里点的表情插进文字，不写就没有输入框的表情按钮，写法不合要求当没写。`textPattern` 的第一个括号认出的是 `textTemplate` 里占位的那一项：写法用 `{type}` 时是表情编号，用 `{name}` 或没写写法时是名字 |

`actions` 用下面的声明式规则写，由宿主用通用代码计算消息可用的操作：

```json
{
  "reply": true,
  "thread": false,
  "react": false,
  "recall": { "own": true, "withinSecs": 120 },
  "edit": null
}
```

- `recall` 或 `edit` 为 null，表示不能做。`own: true` 表示只能对本人发的消息做。`withinSecs` 是平台的时限：过了时限宿主就不再显示这个操作，但最终能不能做，仍以平台的答复为准。
- 已撤回的消息和系统通知，一律没有任何操作。
- “编辑”只对内容是文字和提及的消息显示。编辑框里的原文由宿主按通用规则从消息里拼出来，文字和提及分开成几段。
- v12：`edit` 加 `perMessage: true` 时，每条消息自己说能不能编辑——本人能编辑的那条带 `editDraft`（§7.4，编辑框里的原文，表情、链接按平台的写法），不带就不显示“编辑”。

外部程序的平台说明**刻意不设**的几项（以免以后误加）：

- **没有头像身份规则**：头像用不透明的键（§7.3），同一个头像的键必须保持不变。
- **没有页面可以直接加载的图片来源**：所有图片都经宿主的文件库读取。

### 5.2 能力（`capabilities`）

没写出来的都算不支持。宿主只调用声明过的方法；如果宿主调用了未声明的方法，那是宿主的错误。

```json
{
  "conversations": { "list": true, "search": false, "members": false, "findDirect": false },
  "messages":      { "list": false, "get": false, "forwarded": false },
  "people":        { "get": true, "search": false },
  "send":          { "text": true, "mentions": false, "mentionAll": false, "image": true, "file": false, "lookup": false },
  "recall": false,
  "edit": false,
  "reactions":     { "list": false, "add": false, "remove": false, "catalog": false },
  "resources":     { "read": true, "avatars": true },
  "events":        { "messages": true, "deletions": true, "reactions": false, "conversations": true, "people": true },
  "retranslate":   true,
  "session":       { "renew": false }
}
```

- v12：`conversations.recentActivity` 为真时程序能回答 `conversations/recentActivity`（会话发现，§8）；`session.renew` 为真时程序能用存着的凭据续期（`session/renew`，§6.6）。
- `people.contactActions` 是可选群成员名片能力，缺省关闭。请求绑定来源群与用户编号，插件核验双方仍是群成员；不能添加本人。`contacts/profile` 只读真实关系，不以本机记录缺失、查询错误或不完整分页推断非好友。`contact` 必须给出非空私聊编号且 `canAdd=false`；其他状态不带私聊编号，本人也不可添加。`contacts/add` 仅由用户明确操作触发，不自动重试；重复添加须复用同一私聊。写入结果不确定时先重新查询关系。名片与添加不会发送消息。类型及校验在 `hhh-im-protocol/src/contacts.rs`。
- `people.contacts` 是可选好友目录能力，省略即不支持。读取独立于会话发现与聊天时间：包含没有消息、只有旧消息的既有好友，不创建关系、不读取消息、不列旧群。游标不透明，分页失败重试同一页；插件不能以跳过失败好友冒充完整页。宿主把资料恢复到列表，用户打开或近期消息发现后才启用正常消息读取。未声明的旧插件不会收到 `contacts/list`。
- v15：`people.bots` 为真时程序能按编号读机器人与应用（`people/get` 的 `bots`），要先声明 `people.get`；不写时宿主把机器人一律记成“没有”、不为它们问程序。`people.batchMax`（1–2000）是一次 `people/get` 最多问几个，平台一次读一大批更省时写（如读一遍整份成员名单就认出一大批人）；不写是 100。

- `messages.forwarded` 为真时，插件能把平台明确提供的合并转发或消息快照翻成 `forwarded` 块；默认 false，旧插件不受影响。声明它须同时能列消息、取单条或接收消息推送。它只表示读取，不表示能发送转发记录或读取任意原会话。
- v35：`account.hostAvatar` 为真时程序收宿主交来的本机用户头像（`account/avatar`，§6.8），拿去设置本人在这家 IM 里的头像；不写时宿主不交，线上不出现 `account`。

- v32：`perf` 为真时程序能自报内部耗时（§11.6）；不写时宿主不发任何 `perf/*`。

- `retranslate` 为真时，程序随消息附带去凭据化的原始数据（§7.4 的 `raw`），宿主存一张原始数据表；程序翻译规则升级后，宿主调 `messages/retranslate`（§10.1）让它把本地旧消息重新翻译一遍。为假时不附带原始数据、也不会被调用重译。
- `messageMaintenance`（可选，默认假）允许宿主把已存的统一消息交给程序做本地修正或重读判定，不依赖 `raw`，不替代 `retranslate`。旧程序省略此能力照常接入。
- `agentAttribution: {send, receive}`（可选）分别声明跨账号发送和接收伙伴署名，规则见 §7.4a；两项默认 false。send 要求可发文字，receive 要求可读取或接收消息。
- `richSend: {formats, edit?}`（可选）声明 §7.8 的格式发送；`formats` 取 `strong`、`emphasis`、`underline`、`delete`、`inlineCode`、`link`、`blockquote`、`list`、`code`。要求 `send.text`，`edit: true` 另要求 `edit`。未知格式名忽略；省略或格式集合为空时不收格式正文。协议主版本不变。

- **只读连接**：`send` 为 null，或者其中各项都是 false。宿主不显示输入框，伙伴不在这个连接上回复，所有发送请求都在本地直接拒绝。
- `history=server` 时，`messages.list` 必须为真。`history=none` 时，`events.messages` 必须为真。
- `history=none` 而且 `conversations.list` 为假时，`events.conversations` 必须为真，因为会话清单只能靠推送得到（§11.4）。
- 能力组合不自洽时，宿主拒绝连接，并说明是哪一项。

## 6. 登录、凭据与设置

### 6.1 登录流程

宿主调用 `login/start`，程序返回 `{ "flowId": "<编号>" }`。参数在没声明登录方式时是 `{}`；声明了登录方式（§6.1a）时是 `{ "method", "values", "callbackUrl"?, "state"? }`：这次选的方式、填的内容（`relayUrl`；`appId`、`appSecret`——`appSecret` 只在用户这次填了时给，没填表示沿用程序凭据里的）、宿主收回调的完整地址与一次性 `state`（声明了 `callbackPath` 时给）。旧程序收到多出的字段照常忽略。之后程序发 `login/step` 通知，告诉宿主当前该显示什么，宿主用通用组件显示出来：

| `step.kind` | 字段 | 宿主显示什么 |
|---|---|---|
| `qrCode` | `image`（PNG 的 base64，不超过 512 KiB）、`expiresAtMs`、`hint` | 二维码和倒计时。码过期后由程序自己换新码，再发一次这个步骤 |
| `openUrl` | `url`（https，或本机回环地址的 http）、`label` | 一个“打开浏览器”按钮，不在页面里嵌入网页 |
| `loginWindow`（v36） | `url`（同 `openUrl`）、`label`、`capture`（要截下的跳转开头，1–4 项） | 桌面版开一个无痕登录窗口显示 `url`，截到跳转后经 `login/callback` 交给程序、窗口里换成宿主的收据页，登录有了结局再关窗；登录框里一颗把窗口叫到前面 / 重新打开的按钮（文字用 `label`）。见本节末尾“登录窗口” |
| `confirmOnDevice` | `message` | 等待提示，例如“请在手机上确认” |
| `input` | `fields: [{id, label, secret, placeholder, pattern}]` | 一个表单。用户提交后，宿主调用 `login/submit {flowId, values}` |
| `message` | `text`、`tone`（`info` / `warn`） | 一段说明文字 |
| `showText`（v13） | `caption`、`text`（至多 4096 字）、`layout`（`words` 编号的词 / `code` 一行大字 / `line` 一行等宽）、`copy`、`ack`（勾上才能点确认）、`confirmLabel`（不给就没有确认）、`waiting`（文字下面转圈写这句话） | 出示要保存或要对照的文字：恢复密钥、新电脑上的几个词、好友链接 |
| `list`（v13） | `caption`、`rows: [{id, title, detail, tag, icon: person/device, actions: [{id, label, danger}]}]`（至多 200 行，每行至多 4 颗按钮）、`select: {min, max, confirmLabel}`（多选）、`empty`（没有行时写什么） | 一列东西：每行可带按钮，或多选后确认 |
| `confirm`（v13） | `caption`、`person: {name, detail}`、`sections: [{title, text}]`、`text`、`confirmLabel`、`danger`（确认按钮用危险色） | 确认或说明：核对是谁、不可撤销的确认、几段说明 |

v13 起每一步都可以带 `links: [{id, label}]`（另一种做法，宿主画成底部一行左边的文字按钮、几种上下排，这一步的按钮在右边，至多 8 个）；`input`、`showText`、`list`、`confirm` 还可以带 `cancelLabel`（左边按钮的文字）、`cancelChoice`（左边按钮代表一个选择，如批准框的“拒绝”，不等于关掉）、`noCancel`（没有左边的按钮：这一步不能跳过，点遮罩、按 Esc 都不关）；`input` 另可带 `caption`（字段上面的一句说明）与 `submitLabel`（提交按钮的文字）。用户在一步里做了选择——点了 `links`、行尾的按钮、多选或确认一步的确认、`showText` 的确认、`cancelChoice`——宿主发 `step/choose {flowId, choice, row?, selected?}`（宿主→程序通知，同 `login/submit`、`login/cancel`；登录与 §6.7 的操作共用）：`choice` 是 `confirm`、`links` 或行尾按钮的 `id`、或 `cancelChoice`；`row` 是哪一行的按钮，`selected` 是多选确认时选中的行。程序之后照常推下一步或结束。每段文字（说明、标题、细节、按钮文字、小标题与说明）至多 200 字，超出 §14 上限、编号空着或重复的一步宿主不显示，按这次流程失败处理。

除 `message` 外，每种步骤都可以带可选的 `note: { "text", "tone" }`（`tone` 是 `info` / `warn`，与 `message` 同色调）：程序想在这一步提醒用户什么（例如“扫码后在手机上勾选同步最近的消息”），就在这一步带上，宿主把它画在这一步内容下面、按钮上面；不带就没有。宿主不解析提示的含义，文字原样显示、不支持格式；空文字或超过 §14 上限的提示不显示（这一步照常），不认识的色调按 `info`。旧宿主忽略这个字段。

登录流程以程序发出的两个通知之一结束：`login/finished {flowId, account}` 或 `login/failed {flowId, message, retryable}`。用户中途取消时，宿主发 `login/cancel {flowId}`。v13 起两者都可以带给人看的文字（登录框开着时才用得上）：`login/finished` 的 `title`、`message`（成功页的标题与一段话，不给写“<名字>已登录”“开始同步会话。”），`login/failed` 的 `title`、`retryLabel`（失败页的标题与重试按钮的文字）。

- 同一时间只有一个登录流程。新的 `login/start` 会取消还没结束的旧流程。
- 需要浏览器授权回调（OAuth）的，有两种做法：程序自己在本机回环地址上监听，并把这个地址写进 `openUrl` 步骤里；或者（v12）在登录方式里声明 `callbackPath`，由宿主在本机地址上收（§6.1a）。
- 程序必须在收到 `credential/save` 的成功回复之后，才能发 `login/finished`。
- `openUrl` 一步到了，宿主自动打开它（桌面版交给系统默认浏览器），登录框里的次按钮（文字用 `label`）能再打开一次（v12）；打开的登录页宿主不关，登录结束后由用户关（friend/im-auth-relay v8 起，此前桌面版在授权窗口里、结束后关窗）。
- **登录窗口**（v36）：登录走完时平台跳到一个系统浏览器截不住的链接（如 `slack://…/magic-login-sso/<码>`，系统会把它交给对应的 App）时用这一步。
  - `capture` 每项二选一：自定义协议的 `<协议>://…`（协议不能是 http、https、file、javascript、data、about、blob、ws、wss、ftp、view-source），或 https 开头且带地址与至少一段路径（如 `https://app.example.com/t/`，只到地址的不行）；每项至多 200 字。比较时协议名与地址（`://` 之后、第一个 `/` 之前）不分大小写，其余按字面前缀。`url` 不合规、`capture` 为空 / 超过 4 项 / 有不合格的项，这一步宿主不显示，这次流程按失败处理（同 v13 超出上限的步骤）。规则在协议包 `login_window` 模块（`start_url_ok`、`capture_ok`、`captured`），宿主、自检工具与程序共用。
  - 桌面版宿主一到这一步就开窗（同一步只自动开一次），一次至多一个窗口；窗口无痕、与宿主自己的浏览数据隔离、关掉即清空；窗口里只放行 http/https，网页要开新窗口的在同一个窗口里打开，别的协议一律拦下；窗口没有任何宿主权限。页面要跳到对得上 `capture` 的地址时，宿主取消这次跳转，窗口里换成宿主自己的收据页（“<名字> 授权结果已收到，正在完成登录”，v37），发 `login/callback {flowId, url}`（`url` 是完整地址，与 §6.1a 本机回调同一个方法），每一步至多一次。程序核对后照常以 `login/finished` 或 `login/failed` 结束；对不上这次登录的链接应以 `login/failed` 说明原因。
  - 用户自己关了窗口不算取消：登录框写“登录窗口已关闭”，按钮重新打开（从 `url` 重来）；截到以后关掉收据页不影响登录。登录有了结局、程序推了别的步骤或新的 `loginWindow`、程序退出时，宿主关窗（截到以后至少让收据页停 1 秒）。
  - 截取规则只在宿主，推给页面的那一步不带；`url` 与截下的地址不进日志。宿主不注册任何协议处理器、不改系统的默认打开方式。
  - 宿主不告诉程序自己能不能开窗：开不了时登录框写“登录窗口没能打开”，按钮再试一次。旧宿主读不懂这一步，这次登录会失败；用它的程序依赖含这一步的 SDK（`minHost` 由打包工具自动写），旧宿主装不上。

### 6.1a 登录方式（v12，可选）

`initialize` 结果里的 `loginMethods` 声明程序支持的登录方式，按显示顺序。不写就是只有一种：点“登录”直接走程序推来的步骤。写了时，点“登录”先到登录框的“选登录方式”一步：两种以上用页签选，打开时停在现在在用的方式、带出已保存的内容；填得不对宿主就地说原因、不开始。

```json
[{ "id": "relay", "kind": "relay", "label": "通过中继登录", "note": "…", "placeholder": "https://relay.example.com",
   "defaultValue": "https://relay.example.com" },
 { "id": "own", "kind": "ownApp", "label": "使用自己的应用", "note": "…", "idLabel": "App ID", "idPlaceholder": "cli_ 开头",
   "secretLabel": "App Secret", "secretPlaceholder": "…", "secretHelp": "开放平台 → 凭证与基础信息。只保存在本机。",
   "callbackPath": "callback",
   "callbackHelp": "在应用后台把这个地址登记为重定向地址。",
   "createLink": { "label": "前往开放平台创建", "url": "https://…", "after": "，创建后把 App ID 与 App Secret 填在下面。" } }]
```

| `kind` | 登录前要填 | 宿主核对 |
|---|---|---|
| `relay` | 中继地址（`values.relayUrl`） | 必填；https 网址，或本机回环的 http；只填网址本身 |
| `ownApp` | App ID、App Secret（`values.appId`、`values.appSecret`） | App ID 必填；第一次用或换了 App ID 要填 Secret，不填沿用已保存的 |
| `direct` | 不用填 | 点“登录”后照程序推来的步骤走 |
| `host`（v13） | 不用填 | 只给宿主允许的程序（Friend：带官方签名的）；宿主在 `values.hostToken` 给当前宿主账号的访问令牌、在 `hostAccount: {name, detail, avatarUrl?}` 给这个账号（名字、个人或组织的一行说明；`avatarUrl` 自 v35 起宿主不再填，本人头像改由 `account/avatar` 交，§6.8），没有宿主账号（游客）时这一类不能点 |

- 叫法、说明、灰字、链接文字由程序写，每段至多 200 字；链接只收 https。至多 8 种，`id` 不重复；宿主不认识的 `kind` 不登记那一种。
- **登录成功才保存**：`login/finished` 且凭据已存好之后，宿主保存这次的方式与填的内容（Secret 进系统凭据库），换了方式就删掉另一种的；失败、取消什么都不动。
- **预填中继**（v14）：`relay` 可以写 `defaultValue`（https 网址）：没保存过中继地址时框里预填它；不写就空着让用户填。
- **App Secret 的说明**（v14）：`ownApp` 可以写 `secretHelp`：去哪找、存在哪，画在 Secret 框旁的 ⓘ 里。
- **用宿主账号登录**（v13）：令牌只在这次 `login/start` 里给一次，宿主当场取（快到期先刷新）、不存、不进日志与 wire 记录；程序要再用就自己放进凭据。宿主不允许的程序声明了这一类，宿主不登记它。声明了 `host`、这个宿主账号下还没登录、也没被用户在宿主里手动退出过的程序，宿主在三个时候自动开始这种登录，不用用户点：刚装好、宿主启动、切到这个宿主账号；自动开始的登录不弹登录框，程序推来 `message` 以外的步骤才弹出来，只推 `message` 就以 `login/finished` 结束时卡片直接变成已连接，失败时不弹窗、卡片写程序给的原因。用户手动退出后不再自动开始，点“登录”时登录框写着当前宿主账号，点了才开始。
- **本机回调**：`ownApp` 声明了 `callbackPath` 时，写相对路径（v14，如 `callback`）宿主就拼在这个 IM 自己的 `/hhh-friend-im/<平台编号>/` 下面，程序不必知道宿主怎么给它编号；照 v12 写以 `/` 开头的完整路径时，必须已经在这个目录下面（不合要求时宿主不登记这种方式）。宿主在 `http://localhost:<Friend 端口><callbackPath>` 收：只在这个程序正在登录、路径对、`state` 对得上时，用 `login/callback {flowId, url}`（宿主→程序请求，`url` 是带查询串的完整地址）交给程序，页面是宿主的收据页“<名字> 授权结果已收到”（v37 起，同中继的收据页；此前是一行纯文字）；其余一律拒绝，页面写“这次登录已经结束”。程序核对后照常以 `login/finished` 或 `login/failed` 结束。

### 6.2 凭据由宿主保存

凭据对宿主来说是一串不透明的字符串，宿主只负责存和还，不解读内容。程序要把重新上线所需的全部材料都放进凭据里，例如登录密钥、设备编号、续期材料。

### 6.3 credential/save（程序→宿主请求）

参数：`{ "credential": "<不透明字符串，不超过 64 KiB>", "renewAtMs"?: 0 }`。宿主把它写进系统凭据库（沿用 Friend 现在的分片保存），写好后回复空结果。写失败时宿主回复错误，这时程序不能丢掉旧凭据。`renewAtMs`（v12）是下次要续期的时间，见 §6.6。

- 这些时候都要调用：登录成功、续期后换了密钥、平台轮换了会话材料。
- **先存后用**：要换凭据时，程序先调用 `credential/save`，等宿主回复成功之后，才能丢掉旧凭据。
- 宿主不把凭据写进日志、wire 记录或 HHH 的配置文件。

### 6.4 程序设置项

`initialize` 结果里的 `settings`，声明程序可配置的参数：

```json
[{ "id": "clientId", "label": "Client ID", "secret": false, "required": true, "pattern": "^[0-9.]+$", "help": "在 Slack 应用后台复制" }]
```

宿主在该 IM 的“管理 → 参数配置”中通用生成表单；无声明时不显示入口。缺少 `type` 时沿用文字项，`secret:true` 始终使用密钥处理；协议主版本仍为 1。

| 可选字段 | 形状与规则 |
|---|---|
| `type` | `text / textarea / number / switch / select / secret`；未知类型明确拒绝 |
| `default` | 字符串；数字以字符串传值，开关只用 `true / false`；密钥不能给默认值 |
| `min / max / unit` | 数字上下界（整数）与显示单位 |
| `options` | 单选的 `[{value,label}]`；值唯一，最多128项 |
| `group` | `{id,label,fold?,help?,note?}`；fold 默认折叠，help 为展开摘要，note 始终显示；同组说明一致 |
| `summaryLabel` | 折叠摘要使用的短标题 |
| `showWhen` | `{field,equals}` 简单字符串相等条件；不能引用密钥或自己；隐藏不丢值 |
| `disabledReason` | 不能编辑时显示的原因，宿主也拒绝变更 |
| `notLessThan / notGreaterThan` | 其它数字字段 id 的顺序关系，可引用数字型单选 |
| `pattern` | regex-lite 支持的便携正则（不支持回溯引用／环视）；服务端校验，前端不执行插件表达式 |

- 参数声明最多128项、整体最多1MiB，字段值与长说明最多8192个UTF-8字节，名称最多512字节；声明只含数据，不执行 HTML、JavaScript 或任意表达式。
- 普通值保存在用户参数配置，密钥只存系统凭据库，读取配置只返回已保存标记。空密钥输入保留原值，清除必须显式指定。
- 插件参数显式保存；开着的插件保存后重新连接，关着的只保存。登录完成后的 `session/open` 同样传入当前参数。
- `session/open.settings` 保持字符串映射，只包含当前声明的项；宿主补默认值并校验，秘密到此才物化为明文，不写日志或普通文件。
- 程序升级后，`id` 与类型兼容的项保留原值；删除或类型变化保留旧存储、明确提示校验问题，用户检查后保存。

### 6.5 授权失效与断开

- 程序发现授权失效时，发通知 `session/expired {reason, message}`，`reason` 是 `expired`、`revoked`、`credentialInvalid` 之一。在重新登录之前，程序对之后的请求一律回授权失效错误（§12）。宿主据此停止读取，并提示用户重新登录，走 hhh-im 现有的 `AuthExpired` 流程。宿主保留登录记录与账号名，卡片写“账号 · 登录已失效”，下面一行是 `message`，右边“重新登录”；会话页、添加关注页在“登录已失效”下面也写它（v12）。
- `session/notice {notices: [{id, text, relogin}]}`（程序→宿主通知，v12）：这个连接现在有的问题（如缺某个权限），**每次给全**，空数组就是都好了。宿主把它们画在卡片下面，每条一行；`relogin` 为真的要重新登录才能好，卡片右边多一颗“重新登录”。至多 20 条，每条至多 200 字。
- **请用户处理的事**（v13）：每条还可以带 `action: {id, label}` 与 `prompt: true`。带 `action` 的画成待处理行（品牌黄，不是出错），右边一颗按钮写 `label`（至多 20 字），点了宿主叫 `actions/start {actionId: action.id}`（§6.7；这个编号不必出现在 `initialize.actions` 里）；`prompt` 为真的新提醒出现时，宿主窗口在前台、也没有别的对话框开着，就直接开始这个操作，关掉没处理完的留在卡片上。处理完、作废了，程序在下一次 `session/notice` 里去掉这一条。
- 用户在 HHH 里断开连接时：
  1. 宿主调用 `session/logout`。程序尽量在平台上登出，然后丢掉内存里的会话。
  2. 宿主删除凭据，停止进程。
  3. 本地聊天记录和文件按 HHH 现有规则保留，断开不删数据。

### 6.6 续期（v12，可选）

- 凭据里的会话材料会过期、要定时换新的平台，存凭据时带 `renewAtMs`，并声明能力 `session.renew`。到时宿主叫 `session/renew {credential}`（宿主→程序请求，`credential` 是宿主存着的那份）：程序在跑就直接叫；没在跑时宿主临时起进程、`initialize` 之后不开账号直接叫，续完关掉。程序续完先 `credential/save` 存新的（带下次的 `renewAtMs`），再回空结果；续不了回错误（授权失效回 `authExpired`）。
- 续期、登录保存与退出登录在宿主里串行：退出登录、卸载之后迟到的 `credential/save` 一律回错误。
- `session/open.credentialReadOnly` 为真（凭据是借来的）时宿主不排续期。

### 6.7 插件操作（v13，可选）

`initialize` 结果里的 `actions` 声明程序自己的操作，按显示顺序：

```json
[{ "id": "devices", "label": "我的电脑", "scope": "account" },
 { "id": "members", "label": "群成员", "scope": "conversation", "conversationKinds": ["group"] }]
```

- `scope` 是 `account`（放进连接卡的“管理”菜单，排在宿主自己的项前面）或 `conversation`（放进会话菜单；`conversationKinds` 限定 `direct`、`group`、`channel` 里哪几种会话才有，不写就是每种都有）。会话菜单是宿主的：里面有什么完全由会话所属的程序声明，一项都没有就没有菜单。宿主打开菜单时只按会话种类筛，不问程序；谁能做由程序在点开后的步骤里管。
- 账号级操作可带 shortcut:true（默认 false），请求宿主在日常会话区的＋中按声明顺序展示；会话级和未知 scope 不允许带 true。管理菜单中的既有入口仍保留。
- 至多 16 项，`id` 照程序编号的规则、不重复，`label` 至多 20 字；不认识的 `scope` 那一项不登记。菜单项不单独上色，不可撤销的操作由程序在确认那一步用危险色。
- 用户点了，宿主叫 `actions/start {actionId, conversationId?}`（会话级操作给平台会话编号），程序回 `{flowId, title?}`（`title` 是对话框的标题，至多 200 字，如提醒上的“查看”点开后写“批准新电脑”；不给时宿主用操作的名字，会话级的写“操作名 · 会话名”），之后发 `action/step {flowId, step}` 一步步走（步骤与登录同一套，§6.1），以 `action/finished {flowId, title?, message?, removeFromList?, openConversationId?}` 或 `action/failed {flowId, message, retryable, title?, retryLabel?}` 结束：`action/finished` 给了 `title` 或 `message` 时宿主换成成功页，都不给就直接关掉；能重试时重试就是再开始一次这项操作。`removeFromList: true`（v19）只对会话级操作有效：做完后宿主把这个会话从列表里拿掉，等同取消关注——关注内容与本机记录都留着、平台上的会话不动，这个会话以后有新消息时照常回到列表；账号级操作带了宿主不理会。表单一步提交是 `action/submit {flowId, values}`，用户关掉是 `action/cancel {flowId}`，选择是 `step/choose`（这三个同 `login/submit`、`login/cancel`，都是宿主→程序通知）。
- action/finished 可带 openConversationId：当前账号内操作实际完成的会话编号，遵守 §7 的编号规则。宿主用同一账号连接重新读取并核对会话，再保存关注、打开结果；已有关注保留配置。取消或别的流程不打开。读取或保存失败时操作仍算成功，只提示暂时未打开，不能重试创建。removeFromList 同时存在时优先移除。旧宿主忽略新增字段，旧插件继续原有行为。
- 宿主用登录框同一个对话框，标题是操作的名字（会话级的写“操作名 · 会话名”）。同一个程序同一时间只走一个流程（登录或操作）；新的 `actions/start` 或 `login/start` 取消还没结束的旧流程。

### 6.8 本机用户头像（v35，可选）

- 声明了 `account.hostAvatar` 的程序，宿主在会话上线后、以及用户换了头像（或用默认头像时改了名字）之后，叫 `account/avatar {avatar}`（宿主→程序请求）：`avatar` 是 `{mime, data, sha256, size}`——`mime` 只用 `image/png` 或 `image/jpeg`，`data` 是图片字节的 base64，`sha256` 是 64 位小写十六进制，`size` 是字节数；宿主已经缩到边长 256 以内、256 KiB 以内。`avatar: null` 表示这个用户没有任何头像可交。
- 程序收下回空结果；拿去做什么（设成本人在这家 IM 的头像、发给好友）由程序决定。同一张图可能重复收到（重连、内容没变），按 `sha256` 自己去重。处理失败回错误，宿主只记日志，下次上线或下次变化再交。
- 连续几次变化宿主只交最后一张；没声明的程序永远收不到。

## 7. 数据模型

所有字段用 camelCase。时间一律是 Unix **毫秒**。编号都是不透明字符串：1–256 字节，不含控制字符，在这个账号内稳定不变。

### 7.1 Account

`{ "id", "idType"?, "name", "avatar"?: AvatarKey, "tenant"? }`

`tenant` 是账号所在的工作区 / 租户的名字（v12 起宿主显示它）：卡片的账号行写“账号 · 工作区”。

**编号种类 `idType`**（v14，可选）：账号、发送者、人员、提及（含格式正文与卡片里的人）都可以带，说这个编号是平台的哪一种（如飞书的 `open_id`、`app_id`，系统通知可以是空串）。宿主照样记进本机的身份里，不写时记成 `ext`。顶替内置平台的程序要写成与内置时代相同，本机旧记录里的人与新消息才认得出是同一个。

### 7.2 Conversation

`{ "id", "kind": "group|direct|channel|unknown", "name", "avatar"?, "peer"?: "<私聊对方的用户编号>", "private"?: bool }`

`private` 只在私有频道时为真。

### 7.3 Person 与 AvatarKey

- Person：`{ "id", "idType"?, "kind": "user|bot", "name"?, "avatar"? }`。
- AvatarKey 是不透明字符串，宿主拿它调用 `avatars/read`。同一个头像的键不能变；头像换了，键也必须跟着换。

### 7.4 Message

```json
{
  "id": "…", "conversationId": "…",
  "sender": { "id": "…", "idType": "…", "kind": "user|bot|system", "name": "…" },
  "createdAtMs": 0, "updatedAtMs": 0, "editedAtMs": 0,
  "deleted": false, "incomplete": false,
  "content": [ "Block…" ],
  "replyTo": "<被回复的消息编号>",
  "thread": { "id": "…", "rootId": "…", "inThread": true, "broadcast": false,
              "activity": { "latestReplyId": "…", "latestReplyAtMs": 0, "replyCount": 0 } },
  "reactions": [ "Reaction…" ],
  "raw": { "format": "<程序自定的格式名>", "data": "<去凭据化的原始数据，字符串或 base64>" },
  "editDraft": [ "Outgoing…" ]
}
```

- 除系统通知（`sender.kind` 为 `system`）以外，`sender.id` 必须有值。群消息要给出真实的发送者，不能用群编号代替。
- 撤回的消息：`deleted` 为真，`content` 为空。撤回后仍留在历史里的平台，照常返回这条消息；`deletedMessagesVanish` 为真的平台，改用 `deleted` 事件（§11.2）。
- `reactions`：只要给出，就必须是这条消息**此刻的全部**表情。不知道全部就省略，不能只给一部分。
- `thread` 的含义同 hhh-im 的话题规则：`inThread` 为假的一律不算话题回复。平台没有话题功能时省略。
- `thread.copy {role: "conversation"|"thread", peerId, threadId?}`（v12）：话题回复“同时发到会话”时平台给出两条（话题里一条、会话里一条）的，两条各带它：`role` 说这条是哪一边，`peerId` 是另一边那条的编号，`threadId` 是话题编号（会话里那条带着，用来找话题第一条）。宿主在会话里那条上写“来自话题”，两条算同一条回复。
- `editedAtMs` 只在平台明确表示“改过”时给出。平台不给“改过”标记的，可以在平台说明里声明 `editsByContent`（§5.1），宿主按内容认。
- `editDraft`（v12）：声明了 `actions.edit.perMessage` 时，本人能编辑的那条给出编辑框里的原文（`Outgoing` 数组，块数预算同发送）；不能编辑就不带。
- `raw` 只在声明了 `retranslate` 能力时给（§10.1）：`format` 是程序自己定的格式名（例如 `wechat-addmsg-v1`），`data` 是去凭据化的原始数据。宿主把它和消息一起存进原始数据表，供以后重新翻译；**不含任何令牌、密钥、会话材料**。单条上限见 §14。

### 7.4a 伙伴代发署名（可选）

`Message.agentAttribution` 与 `messages/send.agentAttribution` 使用同一形状：

```json
{"name":"派派","avatar":"data:image/png;base64,…","automatic":true}
```

- `name` 必填，1–512 个 UTF-8 字节，不能全空白或包含控制字符；`automatic` 默认 false，true 表示这一版实际自动发送。`avatar` 可省略，只能是完整 URL 不超过 24 KiB 的静态 PNG、JPEG、WebP base64 data URL。宿主发送端生成至多 64×64 的小图；没有可用头像时接收端使用默认伙伴头像，不传本机路径、宿主 HTTP 路径、外部地址或 SVG。
- `capabilities.agentAttribution: {send, receive}` 两项默认 false，全 false 时省略。send 要求 `send.text`；receive 要求 `messages.list/get` 或 `events.messages`。宿主只向声明 send 的插件发送这个新参数；未声明的旧插件保持原请求形状。
- send 表示插件能将署名随消息传到接收账号，receive 表示插件能从平台数据还原它。具体平台的编码与解码由插件完成；仅在本机按操作编号记一份映射不算跨账号能力。没有可靠的传递方式时不声明，不以正文尾句猜测代发身份。
- 声明能力的插件须在发送回包、lookup、历史列表、单条读取、推送、话题与会话副本里保留同一快照；编辑只改正文，继续返回原署名，撤回后界面隐藏署名。重试同一操作沿用原快照，不随伙伴后来的改名、换头像更新。
- 接收端不依赖本机运行记录；署名只承载名字、头像和自动状态，发送者仍是平台账号。它不是身份认证、不能授予执行日志访问权，不包含本机任务编号、工作目录或凭据。无此字段的消息沿用普通消息与本机旧记录的显示。
- 损坏、超出预算的署名只丢弃署名；坏头像只丢弃头像，保留有效名字。正文与同批其它消息继续读取。署名不进入正文、提示词、搜索或正文复制；不改变平台原有的正文尾句等其它展示机制。

### 7.5 内容块（Block）

| `type` | 字段 | 说明 |
|---|---|---|
| `text` | `text` | 纯文字，原样给出，不含任何标记 |
| `mention` | `userId`?、`idType`?、`kind`?、`name`?、`everyone` | 经平台结构确认的提及。提及所有人时 `everyone` 为真；`kind`（v14）是 `user` 或 `bot`，不写当 `user`：宿主显示机器人时先用这里的名字，显示人时先用人名册的。提及所有人可带平台对“所有人”的叫法（v15）：`name` 写叫法（如 Slack 的 `here`、`channel`、`everyone`，意思各不相同），宿主照写“@叫法”，`userId`、`idType` 可选、照样记下；不带叫法写“@所有人” |
| `resource` | `resource: Resource`、`name`?、`width`?、`height`? | 图片、文件、表情包 |
| `rich` | `root` | 格式正文，即 mdast 的一个子集，节点见 §7.6 |
| `markdown` | `text` | 平台明确标为 Markdown 的原文，由宿主解析 |
| `card` | 结构见附录 B | 卡片：标题、色调和若干块 |
| `linkCard` | `url`、`title`?、`description`?、`source`?、`thumbnail`? | 整张可打开原文的链接分享，见 §7.5b |
| `forwarded` | `record`，见下文 | 合并转发的只读快照 |
| `unsupported` | `kind`、`label`? | 暂时翻译不了的内容。出现它时，整条消息要标 `incomplete: true` |

宿主遇到不认识的 `type` 时，按 `unsupported` 显示，消息照常保存。

消息身份完整时，无法解析的内容块也在原位置转成 `unsupported`，同条其余内容保留，并标 `incomplete: true`；未撤回但正文为空或格式不完整时留下通用消息占位。实时推送、消息历史、补读与发送/编辑结果共用这一接收边界。`raw` 仍只由插件按 §7.4 提供，不从异常 JSON 自动构造。

**Resource**：`{ "key", "kind": "image|file|sticker", "mime"?, "bytes"?, "durationMs"?, "voice"?, "cover"? }`

- `key` 只在它所在的那条消息里有意义。宿主读资源时，总会带上会话编号和消息编号。
- 语音、视频、音频都按 `file` 给，复用 `capabilities.resources.read` 和 `resources/read`，没有单独的播放能力开关。`name` 是用户可理解的文件名；`mime` 必须匹配交出的字节，`durationMs` 是实际已知的正数毫秒，未知时省略。
- 平台明确标记的语音消息另写 `voice: true`；普通音频文件省略或写 `false`。宿主根据此字段显示语音条，不能按文件名、时长或平台编号猜语音；普通音频按 MIME 或扩展名使用音频文件播放器。省略时兼容旧插件；旧宿主可忽略此字段继续按文件处理。
- 宿主只在用户点击后取资源播放。浏览器不支持的编码仍可保存或回原平台处理；插件不把未经转换的私有编码标成 WAV，也不声明未经实现的资源能力。
- **程序必须交出通用格式的数据。** 平台私有的编码（例如微信语音的 SILK），由程序转成 WAV、MP3、OGG 这类通用格式。

### 7.5a 合并转发的只读快照

`forwarded` 块的 `record` 为 `{title, messages, totalCount?, incomplete?}`。`messages` 保持平台给出的顺序，不按时间重排；每条为 `{id?, sender?, createdAtMs?, content, incomplete?}`，`content` 可再次含 `forwarded`。`sender` 是 `{id?, idType?, kind?, name?, avatar?}`：名字与头像取转发时的平台快照，不用当前群成员覆盖；未知的编号、时间、总数省略，不能用外层信息或当前可读条数冒充。

这些内部编号只标识快照，不是当前会话里的消息、人员、话题或未读事实；宿主不为它们创建发送、编辑、撤回入口。`avatar` 仍是 `avatars/read` 的不透明键。资源键须在整个外层消息中足以定位对应资源，宿主始终把外层会话和消息编号交给 `resources/read` / `resources/describe`，由插件在自己的快照里定位并按同一账号的权限读取，不因知道内部编号就绕开原平台权限。

平台未提供、明确拒绝或插件尚不能翻译的部分保留说明；记录、内部消息与外层消息的 `incomplete` 逐层传播，不能把读取失败显示成空记录。内部未知块保留占位及相邻消息。宿主在反序列化之前经 `forwarded::sanitize_content` 核对树形预算：深度 8、每条记录 200 条消息、整棵树 500 条消息及 2000 个块，合计 1 MiB；超限截断或留下占位，并明确标为部分内容。普通消息与 JSON-RPC 帧仍受 §14 的预算约束。

### 7.5b 链接分享卡片

`linkCard` 的 `url` 必须是完整的 HTTP/HTTPS 网页地址；`title`、`description`、`source` 都是可选的纯文本，不能塞平台 XML 或可执行标记。标题和来源缺失时可用网址的主机名展示。宿主沿用打开网页的入口，标题、封面与空白处指向同一原文。

`thumbnail` 是可选的图片 `Resource`（`kind: image`），通过已有 `resources/read` 读取，不把下载地址或密钥直接交给界面。它归属外层消息，快照内也一样；缺失、无效或读取失败只省略封面，不丢弃文章文字或跳转。宿主登记和缓存此资源，但不把装饰性封面当作独立的消息图片附件。

这与结构化通知 `card` 是并列类型，不改变后者的点击语义。插件使用此新类型前须升级到包含它的已发布 SDK 标签，并由打包工具写入对应 `minHost`；不能用旧 SDK 的最低版本声明新类型。协议主版本不变。

### 7.6 格式正文的节点（v1 固定下来）

`paragraph`、`heading{depth}`、`thematicBreak`、`blockquote`、`list{ordered,start}`、`listItem`、`code{lang,value}`、`table{align,pageSize}`、`tableRow`、`tableCell`、`text{value}`、`strong`、`emphasis`、`delete`、`underline`、`inlineCode{value}`、`break`、`link{url}`、`mention{userId,name,everyone,idType?,kind?}`、`emoji{emojiType}`、`resource{resource,name,width,height}`、`unsupported{kind,label}`、`color{tone}`、`tag{text,tone}`、`person{userId,name,idType?,kind?}`、`time{ms,format}`。

- 人与资源照上面的写法（v14 起宿主照规范收，以前只认宿主内部的写法）：`mention`、`person` 的 `kind` 是 `user` 或 `bot`（不写当 `user`）；`resource.resource` 是 §7.5 的 Resource（`{key, kind, …}`），宿主补上它所在的消息。

- 节点之间的嵌套关系遵守 mdast。宿主对节点数、嵌套深度和文字长度有预算（§14）。
- 不认识的节点按 `unsupported` 处理。

### 7.7 Reaction

`{ "id", "conversationId", "messageId", "operator": Person, "emojiType", "atMs"? }`

### 7.8 发出的内容（Outgoing）

`text{text}`、`mention{userId,name}`、`mentionAll`、`image{key}`、`file{key}`。其中的 `key` 来自 `uploads/finish`。

声明 `richSend` 后还可收到 `rich{root}`，根为 `{type:"root",children:[…]}`，节点复用 §7.6。一条格式正文单独占一条消息，不与附件或其它内容块混发。段落、文字、换行、提及和表情为基础节点，其余按 `formats`；提及仍受 `send.mentions` / `send.mentionAll` 约束。

宿主在交给插件之前剥去未声明格式、保留文字，并核对一条正文最多 2048 节点、24 层及发送字节预算。插件仍须核对平台限制。`richSend.edit` 为真时 `messages/edit` 与 `Message.editDraft` 可使用同一形状；不允许编辑的消息不提供 `editDraft`。没用格式时仍使用原有文字与提及块。

## 8. 读取方法（宿主→程序）

分页游标都是程序给出的不透明字符串。宿主把游标和这次查询的条件绑在一起，不会拿去别的查询里用。

| 方法 | 参数 | 结果 | 需要的能力 |
|---|---|---|---|
| `contacts/profile` | `conversationId`、`userId` | `{relationship: "selfAccount" | "contact" | "notContact", conversationId?, canAdd: boolean}` | `people.contactActions` |
| `contacts/add` | 同上；用户明确点击添加后才发送 | 同上；成功必须是 `contact`，提供已有或新建私聊编号 | `people.contactActions` |
| `contacts/list` | `limit`（1–100）、`cursor`? | `{items: Conversation[], next?}`；只含当前账号已有好友对应的私聊，必须有非空 `peer`，不能是本人 | `people.contacts` |
| `conversations/list` | `limit`（1–100）、`cursor`? | `{items: Conversation[], next?}` | `conversations.list` |
| `conversations/get` | `conversationId` | `Conversation` | 必需 |
| `conversations/search` | `text`（1–50 个字）、`limit`（1–20）、`cursor`? | `{items, next?, notice?}` | `conversations.search` |
| `conversations/members` | `conversationId`、`limit`、`cursor`? | `{items: Person[], next?}` | `conversations.members` |
| `conversations/findDirect` | `userId` | `{conversationId?, notice?}`。只定位已有的私聊，不新建会话 | `conversations.findDirect` |
| `people/get` | `ids[]`、`bots[]`?（合计不超过 100 个；声明了 `people.batchMax` 时不超过它） | `{found: Person[], missing: string[]}`。平台明确说不存在的放进 `missing`；这次没读到的，两边都不放（宿主下次再读）。`bots` 是消息里认出的机器人与应用的编号，只在声明 `people.bots` 时给，读到的种类写 `bot`（v15） | `people.get` |
| `people/search` | 同会话搜索 | `{items: Person[], next?, notice?}` | `people.search` |
| `messages/list` | `conversationId`、`threadId`?、`afterMs`?、`beforeMs`?、`newestFirst`、`limit`、`cursor`? | `{items: Message[], next?}`，每页里按从旧到新排 | `messages.list` |
| `messages/get` | `conversationId`、`messageId` | `Message` | `messages.get` |
| `reactions/list` | `conversationId`、`messageId`、`limit`（不超过 50）、`cursor`? | `{items: Reaction[], next?}` | `reactions.list` |
| `emoji/catalog` | 无 | `{emojis: Emoji[], categories?, tones?, updatedAtMs}` | `reactions.catalog` |
| `conversations/recentActivity` | `sinceMs`、`pagesMax`、`skip?`、`firstPageLimit?` | `{items: [{conversationId, kind, firstMs, latestMs, peerId?, conversation?, firstPage?}], complete}`：这段时间里有人（不算机器人、应用、经接口发的）发过消息的会话，每个会话一项，按最新一条从新到旧；`peerId` 是私聊的对方；`conversation`（v15，可选）是群与频道的资料（§7.2 的 Conversation），程序手边正好有时给，宿主就不再为它读一次 `conversations/get`，私聊不给；翻满 `pagesMax` 页还没到起点时 `complete` 为假。一次至多 1000 项（v12）。`skip`（v43，可选）是宿主已在同步的会话编号，程序可以不核对、不报（报了宿主照旧滤掉）。`firstPageLimit`（v43，可选）是宿主要的第一页至多几条，给了程序才可以附 `firstPage {items, more}`：会话主消息流最新的至多这么多条，不带时间条件，与 `messages/list`（`newestFirst`、同样条数）读到的同一批、同样翻译、页内从旧到新，`more` 为真表示平台上还有更早的；程序核对时正好读到了会话最新的消息才附，不为它多发请求；宿主第一次同步这个会话、本地还没有它的消息时就用这页，不再读一遍。消息内容只经 `firstPage` 交出 | `conversations.recentActivity` |

- `messages/list` 的翻页规则同 hhh-im 的 `MessageQuery`：`newestFirst` 为真时，从最新往更早翻。
- **发送前申请额度**（可选）：程序在 `initialize` 声明 `readRequestGating: true` 后，支持此扩展的宿主给上表读取的参数添加 `_meta: {readPermit: "不透明凭据"}`。程序每次发平台 HTTP 读取前，先反向请求 `requests/acquire {permit: "该凭据"}`，只有收到 `{granted: true}` 才能发送这一条；收到错误、空回执或连接断开时停止，不可自行补发。凭据只属于当前连接里的这次读取，不能保存、跨调用复用或用于写操作。
- 同一次读取逐条等额度；宿主沿用原读取单元的优先级、预付额度和取消状态，等待额度不阻塞其他协议消息，也不套插件请求宿主的普通 30 秒时限。读取结束或取消后凭据立即失效。SDK 插件在唯一读取出口用 `runtime::metering::read_request().await?`，没有 `_meta.readPermit` 的旧宿主自动退回计数模式。
- **许可交付回执**（可选）：支持回执的程序发送 `requests/acquire {permit, receipt: true}`。宿主可返回 `{granted: true, receipt: "不透明回执编号"}`；程序收到后，在开始这条平台读取前立即发通知 `requests/received {receipt: "该编号"}`，不等 HTTP 完成。编号必须是 1–128 字节的字符串；无效时停止读取。宿主同一平台最多保留一份未确认交付的许可，收到回执后从此时刻重新计算这笔读取的秒、分钟窗口，不能重复记账。原调用结束、取消或断连时释放其回执；迟到、重复或未知回执不影响其它调用。旧插件不传 `receipt: true` 时宿主不要求回执；旧宿主回复中省略 `receipt` 时新 SDK 沿用明确许可模式。完整的延迟交付保护需要双方支持回执，协议主版本不变。
- **这次向平台发了几个请求**（v14，可选）：上表每个方法的结果对象都可以多带 `"platformRequests": <正整数>`。没有协商发送前申请额度时，宿主每调一次先记一次平台读取，多出的在结果返回后记进这个平台的读取额度（一次调用至多按 64 个记），之后的读取据此等；不写就是 1。已逐条放行的调用不再按这个字段重复扣额度。
- **读取不能有副作用**：不能借读取在平台上标已读，也不能改变在线状态。本人查看后的写回只走独立的 `reading/mark`；外部阅读事实按下面的可选能力单向并入本地未读计算。
- Emoji 的字段同 hhh-im 的 `EmojiInfo`，只是图片地址换成 `emoji/image` 用的编号。v31：Emoji 可带 `inText`（布尔，默认真），假时这个表情不能按 `emoji.textTemplate` 写进文字（只能点成回应），输入框的表情面板不列它；与 `panel`（能不能点成回应）互相独立。用 `{name}` 写法而表情没有名字的也写不进文字。
- `categories` 的每一项是 `{id, name, icon?}`。v31：`icon` 指定分类标签画宿主图标库里的哪个图标，取 `smileys`、`nature`、`food`、`travel`、`activities`、`objects`、`symbols`、`flags`、`custom` 之一（协议包 `types::EMOJI_CATEGORY_ICONS`）；没写或不在表里时宿主画圆，分类编号本身不决定图标。

### 8.1 阅读能力（可选）

三项能力分别声明，省略任一项就不调用对应方法。类型正本是 SDK 的 `reading` 模块；账号由当前插件会话限定，`scope: {conversationId, threadRoot?}` 的 `threadRoot` 是根消息编号，会话和话题的阅读位置互不推进。

| 声明 | 字段 | 方法 |
|---|---|---|
| `capabilities.receipts` | `conversations` / `threads`；`readers` / `unreaders` 表示能否列名单；可选 `maxAgeSecs` 限制本人消息时间窗口 | `reading/receipts`、`reading/people` |
| `capabilities.markRead` | `conversations` / `threads`；正数 `perMinute` 是账号写额度，与读取额度分开 | `reading/mark` |
| `capabilities.readSync` | `conversations` / `threads`；`watermark` / `individual` 至少一项 | `reading/sync`、协商后的 `reading/changed` |

- `reading/receipts({scope,messageIds}) → {items:[{messageId,readCount?,totalCount?,readers?,unreaders?}]}`：只读当前可见的本人消息。缺少人数或未返回该消息表示未知；完整 `totalCount` 不包含自己，只有平台给出完整分母才计算比例。名单可用性不能超出声明，未读名单不能用群成员减已读名单生成。
- `reading/people({scope,messageId,read,cursor?,limit}) → {people,nextCursor?,complete}`：`read=true` 为已读名单，false 为未读。用户点击打开后才读；关闭取消。`complete=false` 且无游标表示只能给部分名单，不代表名单完整。游标必须前进，完整结果不能还有下一页。
- `reading/sync({scope,messageIds}) → {scope,through?,readMessageIds?}`：`through` 是连续水位；`readMessageIds` 仅包含明确已读的请求内编号。逐条 true 不推出其它消息已读，缺字段、无权限和失败都不能补成 false。纯水位查询允许空 `messageIds`。
- 消息的可选 `readPosition` 与本范围 `through` 使用同一序数空间，是最多 64 字节、无前导零的非负十进制字符串（零写 `"0"`）。插件负责把平台位置精确映射，禁止浮点数、毫秒截断或用当前时间替代。宿主按长度和字典序比较；保留既有位置，按账号/会话/话题单调并入自己的消息库，平台主动标未读不倒灌。
- `reading/mark({scope,messageId,position?}) → {state:"applied"|"unknown"}` 是**写方法**，仅来自本人前台实际看见的确切边界或显式标已读动作。后台、AI、预览、普通读取与收到回读均不能触发；不能在执行时改取“最新消息”。超时或断连后不能确定的返回 `unknown`，宿主只读对账，不自动重放。
- `initialize` 可带 `readingEvents:true`。只有宿主明确协商且插件声明 `readSync` 时，才能发送 `reading/changed`，参数与回读结果相同。旧宿主不接收该新通知；SDK `Host` 负责挡住未协商通知。断线恢复仍须按声明补查。

集中预算见 `budget::{READ_MESSAGES_MAX,READ_PEOPLE_MAX,READ_CURSOR_MAX_BYTES,READ_POSITION_MAX_BYTES}`：单批消息最多 100，名单每页 1–100 人，游标最多 4096 字节，整条响应仍遵守通用字节预算。越界、跨范围、重复编号或未声明事实按协议错误处理；读取失败不改变既有已读事实。

自检：`hhh-im-check <清单> --live --read-only --reading-chat <会话编号>` 只读检查；只有另外显式指定 `--allow-mark-read <确切消息编号>` 且移除 `--read-only` 才允许写这一条。未登录、无样本、未授权的跳过不能当成真实验收通过；话题还须从产品入口验证。

## 9. 资源

### 9.1 读取

| 方法 | 参数 | 结果 |
|---|---|---|
| `resources/describe` | `conversationId`、`messageId`、`key`、`kind`；需 `resources.variants` | `{variants: ResourceVariant[]}` |
| `resources/read` | `conversationId`、`messageId`、`key`、`kind`、`variant?`、`offset`、`length`（不超过 1 MiB） | `{data: base64, total?, sha256?, eof}` |
| `avatars/read` | `key`、`offset`、`length` | 同上 |
| `emoji/image` | `emojiType`、`offset`、`length` | 同上 |

- 排队、并发、超时、重试和落盘都由宿主的下载引擎（hhh-download）负责。程序每次只返回请求的那一段，不缓存整份文件。
- `total` 是整个资源的字节数，不是这一段的长度，能给就尽早给。`sha256` 是平台给的内容指纹（小写十六进制），平台不给就省略。
- 程序声明了 `fileSegmentBytes` 时，宿主按段读，断了可以续传。没有声明时，宿主从 0 开始连续读到 `eof`，中途断了就从头再读。
- 这条消息已经没有这个资源时，回 `notFound`，宿主会换别的消息去取，或者标成“读不到”。暂时读不到时，回 `disconnected`。
- `emoji/image`：声明了 `emoji.unlisted.guess`（v12）时，宿主也会拿目录外的表情类型来问；程序按自己的规律猜，猜不到回 `notFound`，界面写名字。

### 9.1.1 图片预览与原件

插件可声明 `capabilities.resources.variants: true`，必须同时声明 `resources.read`。宿主先调 `resources/describe`，再将选中版本的 `id` 放进 `resources/read.variant`。未声明的旧插件不会被调用新方法，省略 `variant` 继续原有读取行为；旧资源不能被宿主冒称为已验证的原件。

一个 `ResourceVariant` 描述一份实际字节：

| 字段 | 含义 |
|---|---|
| `id` | 资源内稳定、不透明、唯一的版本编号；不含地址、凭据或控制字符，字节预算同普通编号 |
| `preview?` | 可用于显示；缺省 false |
| `original?` | 平台提供的真实原始文件；缺省 false，不允许用缩略图替代 |
| `mime?`、`name?`、`bytes?` | 当前版本的实际格式、文件名和总大小；未知省略 |
| `width?`、`height?` | 当前版本按显示方向的尺寸，必须同时给正整数或同时省略 |

- 同一实际版本可以同时用于预览和原件，只返回一项、同时标记两种用途；宿主共用下载和缓存。
- 最多 16 项、最多一份原件，每项至少有一种用途。空列表表示当前没有可读取版本。校验入口是 `ResourceVariants::validate`。
- 聊天流和查看层只从 `preview` 中按显示长边选择合适档位；下载原图与另存原件只使用 `original`。没有预览或原件时明确返回不支持/不可用，不静默换用途。平台已经生成预览时直接提供，不要求前端解码原件或重复转码。
- 分段流身份必须包含 `conversationId`、`messageId`、`key`、`kind` 和 `variant`，可用 `ResourcesReadParams::segment_key()`；预览与原件的字节、长度、eof 和状态不得串用。
- 版本编号对应的字节不能在同一资源内变成另一档内容。平台有新档位时使用新的编号；宿主记住说明供重启/离线读取，手动重试时重新查询。
- 描述需要查询平台时遵守读取计数/放行规则。插件仍自行鉴权、验证真实消息归属和下载地址；平台地址与密钥不交给前端。

### 9.2 上传（发送附件之前）

1. `uploads/begin {kind, name, bytes}` → `{uploadId}`
2. `uploads/chunk {uploadId, offset, data}`：每段不超过 1 MiB，按顺序发
3. `uploads/finish {uploadId, sha256}` → `{key}`
4. `uploads/abort {uploadId}`：中途放弃

程序负责校验长度和摘要。宿主的发送记录会保存 `key`，重试发送时不再重新上传。`key` 不要求在程序重启后仍然有效：失效了就回 `notFound`，宿主会重新上传。

有些平台的上传接口本身就会生成一条消息（例如微信的图片、视频、语音），没有“先上传拿编号、再单独发送”这一步。这类程序在 `uploads/*` 阶段只把字节暂存进临时目录、返回一个本地 `key`，等 `messages/send` 带着这个 `key` 来时才真正上传并生成消息。对宿主来说流程不变：先 `uploads/*` 再 `messages/send`。

## 10. 动作（宿主→程序）

| 方法 | 参数 | 结果 | 需要的能力 |
|---|---|---|---|
| `messages/send` | `operationId`、`target`、`content: Outgoing[]`、`agentAttribution?` | `{state: "accepted", message}` 或 `{state: "unknown", reason}` | `send` 里对应的项；署名另需 `agentAttribution.send` |
| `messages/lookupSend` | `operationId` | 同上 | `send.lookup` |
| `messages/recall` | `conversationId`、`messageId` | `{}` | `recall` |
| `messages/edit` | `conversationId`、`messageId`、`content` | 编辑后的 `Message` | `edit` |
| `reactions/add` | `conversationId`、`messageId`、`emojiType` | `Reaction` | `reactions.add` |
| `reactions/remove` | `conversationId`、`messageId`、`reactionId` | `{}` | `reactions.remove` |

`target` 是下面两种之一：

- 发到会话：`{type: "conversation", conversationId}`
- 回复某条消息：`{type: "reply", conversationId, messageId, inThread, threadRoot?, alsoToConversation}`

规则：

- **同一个 `operationId` 在一个进程的生命周期里只发一次。** 重复调用时返回同一个结果，不能再发一遍。
- 拿不准平台有没有收到时，回 `unknown`，不要自己重发。宿主不会自动重发结果为 `unknown` 的消息，这是 hhh-im 的现有规则。
- 撤回、编辑和表情的时限与权限由平台决定。平台拒绝时，程序把拒绝原样回给宿主，不要重试。
- 超出能力的请求，宿主在本地就先拦下：比如不能撤回别人的消息，只读连接不能发送。

### 10.1 messages/retranslate（宿主→程序，需要 `retranslate` 能力）

程序的翻译规则升级后，用它把本地库里存着原始数据的旧消息按新规则重新翻译一遍，**不联网**。

- 参数：`{ "rule": "<规则名>", "messages": [ { "id", "conversationId", "raw": { "format", "data" } } ] }`。一批不超过 100 条，宿主按 §14 的预算分批。
- 结果：`{ "messages": [ Message… ] }`，每条是重译后的完整消息；程序不想改的那条可以原样返回，宿主按消息编号覆盖本地记录。
- `rule` 由程序自己定名（例如 `wechat-miniapp-card-v2`），写在平台说明的 `retranslateRules` 里（v12，§5.1）。宿主每次打开时按顺序看：存着原始数据、`format` 对得上（规则写了 `formats` 时）、这个规则还没做过的旧消息，每批至多 100 条交给它；按规则名记“这条已经用这个规则重译过”，同一规则不重复重译；程序换了新规则名才会再被调。不联网、不占平台读取额度；这一批出错不记，下次打开再试。这对应 hhh-im 现有的 `content_rereads` 机制，区别是数据来自程序的 `raw` 而不是平台重读。

### 10.2 messages/maintain（宿主→程序，需要 `messageMaintenance` 能力）

用于没有 `raw` 的旧消息。程序只做纯计算，不联网、不写消息库；线上类型仍是 §7 的 `Message`，不传宿主落库格式、连接编号或凭据。

- 请求：`{ "mode": "repair" | "reread", "rule": "旧规则名", "messages": [Message…] }`。`repair` 的规则须在 `contentRepairs`，`reread` 的须在 `contentRereads`。
- `repair` 结果：`{ "updates": [{ "conversationId", "id", "content": [Block…], "editedAtMs": null | 整数, "incomplete": 布尔 }] }`。只返回要改的消息；正文是完整替换，编辑标记与不完整标记显式给出（`null` 清除旧的错误编辑标记）。其余发送者、引用、话题、原始数据、编辑原文由宿主保留，未改动正文节点的身份与资源来源也保留。
- `reread` 结果：`{ "reread": [{ "conversationId", "id" }] }`。只判定哪些需要向平台重读，宿主随后调用 `messages/get`；不符合条件的不写永久完成标记。
- 不得混用两类结果、返回未请求的消息、重复编号或未知内容类型；空结果为 `{}`。超限报 `limitExceeded`，不截断单条正文或返回半份结果。

规则名沿用旧版已发布名字，以便识别已有完成记录；每组至多 64 条，名字 1–128 个 ASCII 字母、数字、`-_.`，组内不重复，重读名不能与 `retranslateRules` 同名。`repair:` 前缀保留给宿主的恢复进度，任何重译规则也不得占用。

宿主按规则顺序分批处理；每批在数据库锁外请求，短事务核对原快照后同时保存正文与本批进度，失败不推进，重开接着做。一次性修正会扫描尚未完成的旧记录，正常消息读取不做此扫描。单条超过线上预算时保留原样，本次暂缓它的后续规则与重读，其它消息继续；暂缓不算成功，下次打开重试。

降级兼容：维护开始前，宿主同事务给整组未完成规则写旧版认识的跳过标记与新版进行中标记。旧版打开时暂停这一组，不能打乱依赖顺序；新版以进行中标记为准，完成后才清理逐条进度并记录最终统计。已有旧完成标记而没有进行中标记的规则永不重做。

## 11. 推送

### 11.1 events/push（程序→宿主请求）

```json
{ "jsonrpc": "2.0", "id": 41, "method": "events/push",
  "params": { "events": [ "Event…" ], "syncState": "<不透明字符串，可选>" } }
```

- 宿主把这一批事件和 `syncState` **在同一个事务里**写进本地库，写完才回复空结果。这个回复就是确认。
- 程序**同一时间只能有一批在途**，收到上一批的确认后才发下一批。这样顺序是确定的，也天然起到背压作用。
- 收到错误回复，或者 30 秒都没有回复时，程序隔一会儿用同样的内容重发。宿主按编号去重。
- 一批最多 500 个事件。

### 11.2 事件种类

未知事件按 §16 记日志后只跳过该事件，其余事件照常接收。必要的消息身份或推送信封无法解析时，宿主回复 `-32602`，不成功确认、不推进 `syncState`；程序应保留这一批，修正后重试。接收失败的诊断不记录消息正文与不透明原始字段。

| `type` | 字段 | 说明 |
|---|---|---|
| `messages` | `conversationId`、`messages[]` | 新到的或有变化的消息，包括编辑、撤回成标记、话题根消息的变化。同一条消息以最新收到的为准 |
| `deleted` | `conversationId`、`messageId` | 消息从平台上消失了（`deletedMessagesVanish` 为真的平台） |
| `reactions` | `conversationId`、`messageId`、`reactions[]` | 这条消息此刻的全部表情 |
| `reactionAdded` / `reactionRemoved` | 前者 `reaction`；后者 `conversationId`、`messageId`、`reactionId` | 单个表情的增加和取消 |
| `conversation` | `conversation` | 新出现的会话，或者会话资料变了 |
| `conversationRemoved` | `conversationId` | 会话在平台上没有了（比如删了好友、退了群）。宿主只记下这个状态，不删本地记录 |
| `people` | `people[]` | 有人的名字或头像变了 |

### 11.3 连接状态（程序→宿主通知，不需要确认）

`status {state: "online|connecting|offline", reason?}`：程序和平台之间的连接状况，宿主用它显示推送状态。`history=server` 的平台在推送断开时，宿主按 hhh-im 现有规则（`PUSH_FALLBACK_AFTER`）回到定时读取。

### 11.4 两类平台

**`history=server`：消息随时能重读**

- 推送的作用只是更早知道有变化。宿主照 hhh-im 现有规则工作：连上、重连、打开会话时补读一次；推送断了就回到定时读取。
- `syncState` 可以不用。

**`history=none`：平台只推一次**

- 推送是消息的唯一来源。宿主不会定时去读消息，也不会往上翻页补读断档，本地最早的那条就是尽头。
- 程序要把平台的同步位置（例如微信的同步游标）放进每一批的 `syncState`。宿主把它和这一批消息一起存进本地库，下次 `session/open` 时交还给程序，程序从那里接着拉。宿主保证：本地库里保存的 `syncState` 之前的所有事件，都已经写进了本地库。
- 这类平台**所有会话的消息都存进 HHH**，不按同步范围丢弃（HHH 已经不再有“先关注再接收”这一步）。宿主收到某个还没在侧栏出现的会话的推送时，存库并走“发现会话”的路径让它出现在侧栏——沿用其他平台的新会话发现表现。否则这条消息只推一次，丢了就再也拿不到。
- 会话清单靠 `conversation` 事件推送过来，存进本地库，按名字搜索会话在 HHH 本地完成。

### 11.5 log/write（程序→宿主通知，不需要确认）

程序把一条结构化日志交给宿主，宿主并进自己的日志系统（带上程序编号）。

```json
{ "jsonrpc": "2.0", "method": "log/write",
  "params": { "level": "error|warn|info|debug", "message": "…", "fields": { "<键>": "<值>" } } }
```

- `message` 和 `fields` 里**不能含凭据、消息正文、令牌**。宿主按 §14 截断过长的内容。
- 这条通知只是把程序内部的日志透出来，便于排查；程序照常也可以写标准错误输出（§3），两者宿主都收。

### 11.6 性能自报（v32，可选，需要 `perf` 能力）

HHH 的性能剖析开着时，程序可以把自己内部的耗时报给宿主。不声明 `perf` 的程序收不到任何 `perf/*`，行为不变。

| 方向 | 方法 | 参数 |
|---|---|---|
| 宿主→程序（请求） | `perf/start` | `{ "session": <整数>, "intervalMs": <整数，≥ 500> }`：剖析开启时发给每个声明了的在跑程序；剖析期间程序重启后补发 |
| 程序→宿主（通知） | `perf/report` | `{ "session", "seq", "fromMs", "toMs", "spans": [{ "name", "startMs", "durMs" }], "counters": [{ "name", "value" }], "histograms": [{ "name", "count", "sumMs", "maxMs", "buckets": [] }] }`：每个间隔至多一条 |
| 宿主→程序（请求） | `perf/stop` | `{ "session" }`：剖析关闭时；之后同一 `session` 的报告宿主丢弃 |

- `fromMs` / `toMs` 是程序自己的单调时钟，`startMs` 是相对 `fromMs` 的偏移；宿主按收到的时刻对齐，不信程序的绝对时间。
- `counters` 是本间隔的增量；`histograms` 的 `buckets` 按固定边界 `[1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000]` 毫秒，比边界多一个（最后一个装超过 5000 的）。
- `spans` 只报最慢与超过阈值的区间，其余进分布。
- `name` 只能是协议方法名或程序固定的标签（如 `GET /open-apis/im/v1/messages`），不能含会话、消息、人员编号，正文、令牌与 URL 参数。
- 超出 §14 的自报上限时宿主丢掉整条并计数，不当作协议错误、不断开程序。
- 用 SDK `runtime` 的程序自动声明并上报：每个宿主请求的排队与处理耗时、平台请求次数与耗时、等待放行、分段与上传字节、程序内 Tokio 的存活任务与繁忙度。

## 12. 错误

JSON-RPC 的 `error.code` 按下表取值；`error.data` 里写明错误分类和给用户看的说明：

```json
{ "code": 1003, "message": "authExpired",
  "data": { "kind": "authExpired", "userMessage": "微信登录已失效，请重新扫码。", "platformCode": "-13",
            "reauth": "expired", "retryAfterSecs": null } }
```

| code | kind | 宿主怎么处理（括号里是对应的 hhh-im `ErrorKind`） |
|---|---|---|
| -32700 / -32600 | 无 | 协议错误，记进日志。程序反复这样就重启它 |
| -32601 | `unsupported` | 当作不支持。如果这个方法程序声明过，就算程序的错误 |
| -32602 | `invalidRequest` | 参数不合法 |
| 1001 | `unsupported` | 平台不支持这件事（Unsupported） |
| 1002 | `permissionDenied` | 没有权限，附带说明（PermissionDenied） |
| 1003 | `authExpired` | 停止读取，要求重新登录，附带 `reauth`（AuthExpired） |
| 1004 | `rateLimited` | 所有读取一起等 `retryAfterSecs` 秒（RateLimited） |
| 1005 | `notFound` | 平台明确说没有（NotFound） |
| 1006 | `invalidRequest` | 平台拒绝了参数（InvalidRequest） |
| 1007 | `disconnected` | 暂时失败，比如网络问题或平台临时故障，按退避重试（Disconnected） |
| 1008 | `protocolError` | 平台给了看不懂的数据（ProtocolError） |
| 1009 | `cancelled` | 请求被宿主取消了（Cancelled） |
| 1010 | `limitExceeded` | 超出平台限制，比如文件太大（LimitExceeded） |

`userMessage` 用宿主在 `initialize` 里给的 `locale` 来写，不能包含令牌或平台的原始响应。`platformCode` 原样记成文字。`reactions/list` 因为缺权限回 `permissionDenied` 时，`userMessage` 写清要开哪个权限、怎么补（v12）：宿主读不到表情时给用户的提示就用这句话。

## 13. 取消、超时与并发

- 宿主不再需要某个请求的结果时（比如已超时，或会话已移出读取范围），发通知 `$/cancel {id}`。程序应尽快停下，回 `cancelled` 或者干脆不回。之后才到的回复，宿主一律忽略。
- 宿主这边的超时：

  | 请求 | 超时 |
  |---|---|
  | 查询 | 10 秒（同 hhh-im 的 `QUERY_TIMEOUT`，超时后换个连接立刻重读一次） |
  | 读资源的一段 | 30 秒 |
  | 发送、上传一段、撤回、编辑、表情 | 60 秒 |
  | `initialize`、`ping` | 10 秒 |
  | `session/open` | 30 秒 |

- 宿主同时最多有 8 个请求在途。程序可以并发处理，也可以排队处理，但**查询不能被耗时长的下载或同步堵住**，否则会超时。

## 14. 宿主强制执行的预算

收到的数据超出下面的预算时，宿主按协议错误处理，不写进本地库：

| 项目 | 上限 |
|---|---|
| 单条 JSON-RPC 消息 | 4 MiB |
| 每批事件 | 500 个 |
| 每页结果 | 请求里给的 `limit` |
| 每条消息的内容块 | 200 个 |
| 一条外层消息内的转发树 | 深度 8；每条记录 200 条、整树 500 条消息、2000 个块；合计 1 MiB。超限保留可读部分并标不完整 |
| 格式正文、卡片 | 沿用 hhh-im 现有的节点数、深度预算 |
| 单块文字 | 64 KiB |
| `agentAttribution.name` | 512 个 UTF-8 字节，无控制字符、非空白 |
| `agentAttribution.avatar` | 完整 data URL 24 KiB；坏头像回退默认头像，不丢正文 |
| 编号 | 256 字节，不含控制字符 |
| 资源的一段 | 1 MiB，而且不超过请求的 `length` |
| 资源总长 | 不超过 `total`；图片、头像还要不超过 hhh-im 现有的上限 |
| 登录二维码 | 512 KiB |
| 登录步骤的提示 `note.text` | 200 个字（按字符数；超出的提示不显示，这一步照常） |
| logo | 256 KiB |
| 凭据 | 64 KiB |
| 单条消息的 `raw` 原始数据 | 256 KiB |
| 一次 `messages/retranslate` | 100 条 |
| 一次 `messages/maintain` | 100 条，请求参数及结果各不超过 2 MiB；仍受 4 MiB JSON-RPC 帧上限约束 |
| 日志行、`log/write` 的 `message` | 截到 4 KiB |
| 一条 `perf/report`（v32） | 64 KiB；区间至多 256 个；计数与分布合计至多 128 个；名字至多 128 字；`intervalMs` 至少 500。超出整条丢弃并计数，不按协议错误处理 |
| 登录方式（v12） | 至多 8 种；每段文字（叫法、说明、灰字、链接文字）至多 200 字 |
| 登录时填的每个值、回调地址（v12） | 8 KiB |
| `session/notice`（v12） | 至多 20 条，每条至多 200 字（超出的那条不显示） |
| `conversations/recentActivity` 的结果（v12） | 一次至多 1000 项（多出的不用） |
| `conversations/recentActivity` 附的第一页（v43） | 每页至多 `firstPageLimit` 条（多了整页不用，会话照常报）；一次结果里所有 `firstPage` 按 JSON 合计至多 2 MiB（单条消息上限的一半），装不下的项不附 |
| `retranslateRules`（v12） | 至多 32 条（重复的名字不用） |
| 平台说明里的正则（`textPattern` 等，v12） | 1 KiB |
| `editDraft`（v12） | 块数同发送 |
| 操作（v13） | 至多 16 项；`label` 与提醒按钮的文字至多 20 字（多出的项、太长的不登记） |
| 步骤（v13） | `showText.text` 至多 4096 字；列表至多 200 行、每行至多 4 颗按钮；`links` 至多 8 个；`sections` 至多 16 段；其余每段文字至多 200 字（超出的一步不显示、这次流程失败） |
| `account/avatar` 交出的头像（v35，宿主自己守） | 边长 256 以内、256 KiB 以内，只用 PNG 或 JPEG |
| 登录窗口一步（v36） | `capture` 1–4 项、每项至多 200 字；`label` 至多 200 字（不合规的一步不显示、这次流程失败） |

宿主还会核对这几件事：
- 结果里的会话编号和请求的一致。
- 消息属于请求的那个会话。
- 时间在合理范围内：不早于 2000 年，也不晚于当前时间加 1 天。

## 15. 版本与演进

- `protocolVersion` 是整数主版本号。只有不兼容的改动才升主版本，宿主至少同时支持当前和上一个主版本。
- 同一个主版本里只做加法。新增的方法、事件、字段、内容块和能力项都是可选的，并通过能力声明。
- 双方都忽略不认识的字段。宿主遇到不认识的事件，记日志、跳过，这一批仍然照常确认。程序遇到不认识的方法，回 -32601。
- 每加一项，都在本文末尾的“变更记录”里记一行，写明加了什么。

## 16. 最小可接入集

| 组合 | 程序最少要实现 | 在 HHH 里能做什么 |
|---|---|---|
| 只读，能重读历史 | `initialize`、`session/open`、登录、`conversations/list`、`conversations/get`、`messages/list` | 看会话和消息、加关注、伙伴能读；不能发消息 |
| 只读，平台只推一次 | `initialize`、`session/open`、登录、`conversations/get`、`events/push`（`messages` 和 `conversation` 两类事件） | 同上 |
| 能发文字 | 上面任意一种，再加上 `messages/send`（声明 `send.text`） | 再加上发消息、伙伴回复 |

要显示图片和文件，还要实现 `resources/read`；要显示头像，还要实现 `avatars/read`。

## 17. 自检工具 hhh-im-check

HHH 提供一个命令行工具。第三方开发者用它检查自己的程序，HHH 的验收也用它：

```
hhh-im-check <清单路径> [--mock] [--live --read-only] [--allow-send <会话编号>] [--login <登录方式编号>]
```

每一项检查都给出通过、失败或跳过，失败时写明原因：

1. 清单合法；程序在 10 秒内回复 `initialize`；标准输出里没有协议以外的内容。
2. 平台说明、能力和设置项都合法，能力组合自洽（§5.2）。
3. 每个声明了的方法都能调用；没声明的方法不会被调用。
4. 数据规则：
   - 编号稳定（同一条消息读两次，编号相同）；
   - 每页里从旧到新排；
   - 游标最终会翻到头；
   - 时间是毫秒；
   - 普通消息有发送者；
   - 撤回的消息内容为空。
5. 资源：分段读出的总长一致，`eof` 正确，`sha256` 和实际字节对得上。
6. 推送：
   - 同一批重发不会产生重复；
   - 不确认就会重发；
   - 交还 `syncState` 之后，从断点接着推。
7. 取消有效，`ping` 及时回复，`shutdown` 后 5 秒内退出。
8. 发送：只读程序会拒绝发送；拿不准的发送回 `unknown`，并且不重发。这一项需要 `--allow-send`。
9. 凭据：程序先存后用；宿主拒绝保存时，程序保留旧凭据。
10. v12 的新声明（没声明的跳过）：`userAdjustable`、`editsByContent` 的取值宿主认得出；`retranslateRules` 名字不重复；`emoji.textPattern` 能编译、有一个括号；`logoInk` 是合法的外框；`loginMethods` 的类型认得出、编号不重复、不超过 8 种，`callbackPath` 在这个 IM 自己的 `/hhh-friend-im/<平台编号>/` 下面；声明了 `conversations.recentActivity` 就要能回合法结果（v43：自检照宿主的问法带 `firstPageLimit`，附的 `firstPage` 不超过要的条数与合计预算），声明了 `session.renew` 就要能答。
11. v13 的新声明（没声明的跳过）：`actions` 的位置认得出、编号合规不重复、文字不超长、不超过 16 项；声明了账号级操作、登录着时 `actions/start` 要回流程编号；声明了 `host` 登录方式时提示只有宿主允许的（带官方签名的）程序能用。
12. v15：声明了 `people.get` 时，问一个不存在的编号（声明了 `people.bots` 时人与机器人各一个），结果读得懂、同一个编号不会既在 `found` 又在 `missing`；`people.bots` 要先声明 `people.get`、`people.batchMax` 在 1–2000 之间（随能力组合一起查）。
13. v31（没声明的跳过）：`emoji.textTemplate` 合写法规则；声明了 `reactions.catalog` 时读一次目录：同时声明了 `textPattern` 的，能写进文字的每个表情按写法写出来，`textPattern` 要整段认出、认出的名字或编号（按占位）对得回这个表情；分类的 `icon` 要在宿主图标库里。没登录读不到目录时跳过。
14. v35：声明了 `account.hostAvatar` 时交一张 1×1 的 PNG 样图，程序 10 秒内回成功；交头像会改本人头像、发给别人，算写操作，同发送一样需要 `--allow-send`（没给时跳过）。
15. v32：声明了 `perf` 时，`perf/start`（间隔 500 毫秒）之后收到至少一条合规的 `perf/report`（会话编号对、不超上限、不比间隔密），`perf/stop` 之后不再收到。
16. v36：给了 `--login <方式>` 且没在线时，开始一次这种登录，5 秒内要推第一步，每一步宿主都认（读得出来、不超上限；`loginWindow` 的 `url` 与 `capture` 合规），然后取消这次登录。开始登录可能连平台，所以不给 `--login` 时跳过。

`--mock` 要求程序支持不联网的模拟模式（见实现范式 §9）；`--live --read-only` 用真实账号，只做读取。

## 附录 A：协议与 hhh-im 接口的对应关系

| 协议 | hhh-im |
|---|---|
| `initialize` 里的 `platform` | `Platform` / `PlatformInfo`（声明式规则由外部程序适配器执行） |
| `session/open` 里的 `account` | `ImClient::account` |
| `conversations/list` / `search` / `get` | `ImClient::list_conversations` / `search_conversations` / `get_conversation` |
| `conversations/findDirect` | `ImClient::find_direct_conversation` |
| `conversations/members` | `ImClient::list_members` |
| `contacts/list` | `ImClient::contacts`（`has_contacts` 声明，`HubEvent::Contacts` 交宿主） |
| `people/search` / `people/get` | `ImClient::search_users` / `look_up_people` |
| `messages/list` / `messages/get` | `ImClient::list_messages`（毫秒和秒在适配器里换算）/ `get_message` |
| `reactions/list` / `add` / `remove` | `ImClient::list_reactions` / `add_reaction` / `remove_reaction` |
| `emoji/catalog` / `emoji/image` | `ImClient::emoji_catalog` / `emoji_image` |
| `messages/send` / `lookupSend` | `ImClient::send` / `lookup_send` |
| `messages/recall` / `edit` | `ImClient::recall` / `edit` |
| `resources/read` / `avatars/read` | `ImClient::open_resource` / `open_avatar` |
| `uploads/*` | `ImClient::upload_resource` |
| `events/push` | `ImClient::live` → `LiveFeed`（新增：写进本地库后才确认） |
| `messages/retranslate` | `ImClient::content_rereads` → `ContentReread`（新增：数据来自程序的 `raw`，不联网） |
| `messages/maintain` | `ImClient::message_maintenance` / `maintain_messages`，统一消息的本地修正与重读判定 |
| `log/write` | 宿主日志系统（新增：程序把结构化日志透给宿主） |
| `login/*`、`credential/save`、`session/expired` | Friend 的授权和凭据保存（`StoreGrantBackend`）；外部程序走一套通用流程 |
| `conversations/recentActivity`（v12） | `ImClient::recent_activity`（会话发现） |
| `session/notice`、`session/renew`、`login/callback`（v12） | Friend 的插件卡问题行、插件续期任务、本机回调路由 |
| `account/avatar`（v35） | `ProviderProcess::host_avatar`；Friend 在插件上线与用户头像变化时交 |

协议的线上类型和 hhh-im 的内部类型分开定义、分开冻结，由适配器在两边之间翻译。这样内部模型怎么改，都不会影响已经发布的程序。协议里没有连接编号，`connection_id` 和 `ActorRef.id_type` 由适配器补上。

## 附录 B：卡片

结构同 hhh-im `ImCard` 的 JSON：`{title?, subtitle?, tags[], tone, wide, blocks[]}`。

- 块的种类：`paragraph{parts: [{type: "markdown", text}]}`、`columns{columns}`、`divider`、`rich{children}`、`images{columns, images: [{resource, alt?, width?, height?}]}`、`people{people: [{userId, name, idType?, kind?}], size, avatar, name}`、`panel{title, expanded, border, blocks}`。`images[].resource` 是 §7.5 的 Resource，`rich`、`panel.title` 里的节点同 §7.6。
- 色调取值：`neutral`、`grey`、`blue`、`green`、`yellow`、`orange`、`red`、`purple`。
- 这套结构在 v1 固定下来。

## 附录 C：一段完整交互示例

```text
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersions":[1],"host":{"name":"HHH","version":"0.9.0"},"locale":"zh-CN","tempDir":"…","network":{"proxy":null}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"provider":{"id":"wechat-pad","name":"微信","version":"0.3.0"},"platform":{"avatarShape":"rounded","history":"none","actions":{"reply":false,"thread":false,"react":false,"recall":null,"edit":null}},"capabilities":{"conversations":{"list":false},"send":{"text":true,"image":true},"resources":{"read":true},"events":{"messages":true,"conversations":true,"people":true}},"settings":[]}}
→ {"jsonrpc":"2.0","id":2,"method":"session/open","params":{"credential":null,"settings":{},"syncState":null}}
← {"jsonrpc":"2.0","id":2,"result":{"state":"needsLogin","reason":"noCredential","message":"请扫码登录微信。"}}
→ {"jsonrpc":"2.0","id":3,"method":"login/start","params":{}}
← {"jsonrpc":"2.0","id":3,"result":{"flowId":"f1"}}
← {"jsonrpc":"2.0","method":"login/step","params":{"flowId":"f1","step":{"kind":"qrCode","image":"iVBOR…","expiresAtMs":1791090000000,"hint":"用手机微信扫码"}}}
← {"jsonrpc":"2.0","method":"login/step","params":{"flowId":"f1","step":{"kind":"confirmOnDevice","message":"请在手机上确认登录"}}}
← {"jsonrpc":"2.0","id":"p1","method":"credential/save","params":{"credential":"…"}}
→ {"jsonrpc":"2.0","id":"p1","result":{}}
← {"jsonrpc":"2.0","method":"login/finished","params":{"flowId":"f1","account":{"id":"wxid_me","name":"小满"}}}
← {"jsonrpc":"2.0","id":"p2","method":"events/push","params":{"events":[{"type":"conversation","conversation":{"id":"123@chatroom","kind":"group","name":"家人群"}},{"type":"messages","conversationId":"123@chatroom","messages":[{"id":"7012…","conversationId":"123@chatroom","sender":{"id":"wxid_mom","kind":"user"},"createdAtMs":1791080000000,"content":[{"type":"text","text":"吃饭了吗"}]}]}],"syncState":"…"}}
→ {"jsonrpc":"2.0","id":"p2","result":{}}
```

（`→` 是宿主发给程序的，`←` 是程序发给宿主的。）

## 附录 D：旧内置账号交接（`hhh-builtin-v1`，v12）

带官方签名、编号是保留编号的程序还没有自己的凭据，而本机留有同编号的旧授权与设置时，宿主在 `session/open.legacy` 里交接。这里只读取旧格式，不启动旧平台客户端；交接未完成时可在下次打开重试：

```json
{ "format": "hhh-builtin-v1",
  "credential": "{\"accessToken\":\"…\",\"refreshToken\":\"…\",\"expiresAt\":1791090000,\"scopes\":[\"…\"],\"account\":{\"id\":\"…\",\"name\":\"…\"}}",
  "settings": { "mode": "relay", "relayUrl": "https://…", "appId": "…", "appSecret": "…" } }
```

- `credential` 是一段 JSON 文字：内置平台的访问令牌 `accessToken`、刷新令牌 `refreshToken`、访问令牌的过期时刻 `expiresAt`（Unix 秒）、权限 `scopes`，以及上次核验的账号 `account {id, name}`；内置平台没有的项为 null 或省略。
- `settings` 是内置平台在 HHH 连接设置里的那一栏原样（各平台字段不同，如登录方式 `mode`、`appId`、`relayUrl`），其中存在系统凭据库里的密钥换成明文（如 `appSecret`），授权的引用去掉。密钥只经标准输入交这一次，宿主不写日志。
- 程序先经 `credential/save` 保存自己的凭据，收到成功回复后再回 `online`，并用 `adoptedSettings`（§4.2）交回参数和登录设置。宿主确认凭据、参数与登录设置全部落盘后才删除旧引用；未完成则保留旧材料。插件已有凭据但设置交接中断时，可在普通 `session/open` 再返回 `adoptedSettings` 补齐；借用凭据时不转存、不删除来源。关注、本机消息与文件沿用同编号的数据。

## 可选会话分组扩展

此扩展是只读会话导航，不是创建群聊、标星或自动回复规则。协议主版本仍为 1。

- 宿主在 `initialize` 增加 `conversationGroups:true`，表示接受下面的事件；缺省 false。插件仅对声明接受的宿主发送新事件，旧宿主照常接收原事件批次。
- `capabilities.conversationGroups:{list,members,changes}` 各项缺省 false；members、changes 需要 list。未声明的插件不被调用、不显示入口。
- `conversationGroups/list {limit,cursor?}` 返回 `{items:[{id,name,kind,sortKey?}],nextCursor?,incomplete?}`。kind 为 label 或 smart；id 是账号内不透明编号，不能以名字当身份。sortKey 按 Unicode 码点升序，缺省排最后，同键按 id 排序。编号最多 256 UTF-8 字节、名字最多 1024、排序键最多 256。
- `conversationGroups/members {groupId,limit,cursor?}` 返回相同分页信封，items 为完整的 `Conversation`，包括宿主未见过的会话；不能只返回宿主已加载的交集。读取不创建会话、不改归属、不标已读。
- limit 为 1–100。首屏省略 cursor（null 同无游标），后续原样传回。单页最多 100 项，游标最多 8192 UTF-8 字节。只有缺省/null 的 nextCursor 表示结束，空项目页仍可有下一页；空串、重复游标、预算耗尽都应报错。incomplete 标记暂时无法表示的项目，不能默默当成空组。
- `events/push` 新事件 `{type:"conversationGroupsChanged",catalog:boolean,groupIds:string[]}` 只表示失效，groupIds 为空表示所有组。走原确认、重试和取消边界，不新增推送通道。
- 宿主将目录与当前组成员分别做有界完整读取：100 页/1000 组、500 页/10000 会话；全部成功才替换内存，失败保留旧内容可重试。读取中收到失效后再补读一次；进入、事件流重连或 resync 同样补读。账号或来源切换取消旧读，不接受迟到结果。不新增持久化文件或游标账本。
- 旧 SDK 插件可按同一正式 JSON 形状实现增量；SDK 导出/发布仍独立进行，不假称旧标签已包含新 Rust 类型。执行类型见 `hhh-im-protocol/src/groups.rs`，兼容与真实 provider/hub 测试见 `provider_groups.rs`。

## 账号贴纸、表情包与收藏

这些是可选能力，主版本仍为 1。`capabilities.stickers` 的 `packs`、`favorites`、`favoriteAdd`、`favoriteRemove`、`send` 均默认 false、独立声明；没有实现的不能声明。这里列的是账号已有的内容，不包含表情商店或上传管理；不混入 `emoji/catalog` 与消息回应。

| 请求 | 参数 | 返回与门禁 |
|---|---|---|
| `stickers/packs` | `{limit,cursor?}` | `StickerPage<StickerPack>`，须 `packs` |
| `stickers/list` | `{packId,limit,cursor?}` | `StickerPage<Sticker>`，须 `packs` |
| `stickers/favorites` | `{limit,cursor?}` | `StickerPage<Sticker>`，须 `favorites` |
| `stickers/favorite` | `{id,favorite}` | `{}`；目标状态为 true 须 `favoriteAdd`、false 须 `favoriteRemove`；幂等设定目标状态，不是反转 |
| `stickers/image` | `{key,offset,length}` | 同资源分段读取的 `ResourceChunk`；须 `packs` 或 `favorites` |

`StickerPack` 为 `{id,name,cover?}`，cover 为图片键；`Sticker` 为 `{id,name,imageKey,favorite?}`，收藏状态未知时省略；`StickerPage` 为 `{items,next?}`，无 next 表示读完。编号、包编号、图片键、游标均为账号内不透明字符串；稳定发送编号不可拿远端 URL 或上传键冒充。图片变化须换图片键。空、控制字符或超过 `STICKER_ID_MAX`（512 字节）的编号/游标无效；名称也不超过 512 字节。一页 limit 为 1–`STICKER_PAGE_MAX`（100），回包不超过请求上限，next 不得重复当前游标。目录缺权限、读取失败要报错，不伪装空结果；图片沿用宿主表情图片总大小预算。

原生发送使用 `messages/send`，content 只能有一项 `{"type":"sticker","id":"<目录里的发送编号>"}`，须 `stickers.send`，不需要 `send.text`。目标、话题、操作编号和 accepted/unknown 沿用现有发送协议；unknown 不自动重发，不可通过转换为文件上传规避结果语义。有 `stickers.send` 的插件不是只读插件。收藏失败保持原状态，宿主可让用户明确重试。

线上类型在 `src/stickers.rs`、参数在 `src/method.rs`；示例插件提供通用的目录、收藏与发送数据。旧插件不声明时全部关闭；新插件依赖包含这些类型的 SDK 后再接入，SDK 和插件发布独立进行。

## 变更记录

- 2026-10-22：新增可选 `AgentAttribution`、发送参数和消息字段、独立收发能力声明、容错与体积预算；`hhh-im-check --allow-send` 对声明发送署名的插件检查回包保留。协议主版本保持 1；实际插件适配与 SDK 发布独立进行。

- 2026-10-04：v1 草案。补入 `log/write`（结构化日志）、消息 `raw` 原始数据与 `messages/retranslate`（可重新翻译）、`history=none` 平台存全部会话并走发现路径、上传即发送的平台说明、信任边界措辞纠正。
- 2026-10-10（契约 v12，全部为可选的加法，主版本仍是 1）：会话发现（`discovery`、`conversations/recentActivity`、能力 `conversations.recentActivity`、`readDefaults.discover`）；读取与下载的数放开数值范围、`reads` 不再必填、`userAdjustable`；`editsByContent`；`retranslateRules`；`session/notice` 与 `Account.tenant` 的显示、`reactions/list` 缺权限时的 `userMessage`；登录方式 `loginMethods`、`login/start` 的 `{method, values, callbackUrl, state}`、`login/callback`；续期 `credential/save.renewAtMs`、`session/renew`、能力 `session.renew`、`session/open.credentialReadOnly`；`emoji.unlisted.guess`、`emoji.textPattern`、`emoji.whiteBackground`；清单 `logoStyle`、`logoInk`；`actions.edit.perMessage` 与消息 `editDraft`；`thread.copy`；保留编号与交接（`session/open.legacy`、`adoptedSettings`、附录 D）；§5.1“刻意不设”删去“不猜表情图”“不比内容认编辑”两项；§14 新预算；§17 新检查。
- 2026-10-11（契约 v14，全部为可选的加法，主版本仍是 1）：编号种类 `idType`（账号、发送者、人员、提及与格式正文、卡片里的人）；格式正文与卡片里的人与资源宿主照 §7.6、附录 B 的写法收；读取结果可带 `platformRequests`；`loginMethods` 的 `callbackPath` 可写相对路径、`relay` 可写 `defaultValue`、`ownApp` 可写 `secretHelp`；`mention` 块可带 `kind`；`avatarIdentity`；表情的肤色版本（`tone`、`toneOf`）与目录的 `tones`，以及目录的 `categories`，宿主照收（以前丢掉）；`emoji.unlisted.guessPattern`；宿主照声明处理 `conversationNouns`、`emoji.namePattern`、`unlisted` 的占位；平台编号只由宿主定（协议包不再推）。
- 2026-10-12（契约 v13，全部为可选的加法，主版本仍是 1）：登录方式 `host` 与 `login/start` 的 `values.hostToken`、`hostAccount`（宿主账号登录、自动开始）；`initialize.actions` 与 `actions/start`（结果可带 `title`）、`action/step`、`action/submit`、`action/cancel`、`action/finished`、`action/failed`（插件操作，§6.7）；步骤 `showText`、`list`、`confirm`，每一步的 `links`，`input` 的 `caption`、`submitLabel`，`cancelLabel`、`cancelChoice`、`noCancel`，以及 `step/choose`；`login/finished` 的 `title`、`message` 与 `login/failed` 的 `title`、`retryLabel`；`session/notice` 每条的 `action`、`prompt`；“保留编号”改为宿主官方列表里的编号，只有同编号的内置平台存在时平台编号才是编号本身、才顶替；§14 新预算；§17 新检查。
- 2026-10-12（契约 v15，可选的加法，主版本仍是 1）：能力 `people.bots` 与 `people/get` 的 `bots`（按编号读机器人与应用）、`people.batchMax`（一次读人的上限）；宿主照 `people/get` 的结果办——只把 `missing` 记成“没有”，两边都没有的下次再读（以前没返回的一律记“没有”）；提及所有人可带平台的叫法（`mention` 的 `name`，内容块与格式正文里都是）；会话发现的一项可带群与频道的资料（`conversation`）；协议包的插件运行时（`runtime` 部件）与格式正文、卡片的类型（`content` 模块）。
- 2026-10-16（契约 v19，可选的加法，主版本仍是 1）：`action/finished` 的 `removeFromList`（会话级操作做完后让会话离开宿主的列表，等同取消关注；不认识它的旧宿主只关对话框）。
- 2026-10-21（hhh-im client 契约 v37，主版本仍是 1）：声明了 `discovery`、没有 `conversations.recentActivity` 但能列会话与读消息的程序，由宿主逐会话限频发现会话（原先这种声明不生效）；程序不用新增方法。
- 2026-10-22（契约 v31，全部为可选的加法，主版本仍是 1）：表情写进文字的写法 `emoji.textTemplate`（宿主据此给输入框的表情按钮）、`textPattern` 认出的是名字还是编号跟写法一致；表情的 `inText`；目录分类的 `icon`；§17 新检查。
- 2026-10-22（契约 v35，可选的加法，主版本仍是 1）：能力 `account.hostAvatar` 与宿主→程序请求 `account/avatar {avatar}`（本机用户头像：PNG / JPEG、边长 256 与 256 KiB 以内，§6.8）；`hostAccount.avatarUrl` 宿主不再填；§14 新预算；§17 新检查（需要 `--allow-send`）。
- 2026-10-22（契约 v32，可选的加法，主版本仍是 1）：能力 `perf`、`perf/start` / `perf/stop` 请求与 `perf/report` 通知（§11.6）、自报上限（§14）、自检第 15 项；SDK `runtime` 自动声明并上报。
- 2026-10-25（契约 v37，程序不用改）：登录窗口截到跳转后窗口里换成宿主的收据页，登录有了结局再关；本机回调的页面换成同一张收据页；示例插件页面换成收据页版式。
- 2026-10-25（契约 v36，可选的加法，主版本仍是 1）：登录步骤 `loginWindow`（§6.1“登录窗口”：桌面版无痕登录窗口，截到 `capture` 开头的跳转经 `login/callback` 交给程序）；协议包 `login_window` 模块；§14 新预算；`hhh-im-check --login` 与自检第 16 项。
- 2026-10-26（契约 v43，见 hhh-im client 契约 v41；可选的加法，主版本仍是 1）：`conversations/recentActivity` 的参数 `skip`、`firstPageLimit` 与结果项 `firstPage {items, more}`（会话发现读到的最新一页交给宿主当第一次读取）；§14 新预算；自检第 10 项核对附的第一页。协议包 `ActivityItem` 加字段、不再实现 `Eq`，直接构造它与 `RecentActivityParams` 的程序升级时补上新字段。
