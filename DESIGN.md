# WPW 密码管理器架构设计文档

## 目录

1. [项目概述与设计原则](#1-项目概述与设计原则)
2. [项目结构](#2-项目结构)
3. [加密方案](#3-加密方案)
4. [存储格式设计](#4-存储格式设计)
5. [CLI 命令设计](#5-cli-命令设计)
6. [Native Messaging 协议](#6-native-messaging-协议)
7. [浏览器扩展架构](#7-浏览器扩展架构)
8. [安装与分发](#8-安装与分发)
9. [关键安全考量](#9-关键安全考量)
10. [依赖选型](#10-依赖选型)

---

## 1. 项目概述与设计原则

### 1.1 核心目标

本项目为完全本地化的个人密码管理器，所有敏感数据以加密文件形式存储在用户设备上，用户自行选择同步工具（iCloud Drive、OneDrive、Dropbox 等）。系统由三个独立组件构成：

- **CLI 工具**：命令行界面，提供完整的密码管理功能
- **Native Messaging Host**：作为浏览器扩展与本地系统之间的安全桥梁
- **浏览器扩展**：Chrome / Edge 扩展，提供自动填充功能

### 1.2 平台支持

| 平台 | CLI | Native Host | 浏览器扩展 | 状态 |
|------|-----|-------------|-----------|------|
| Windows 10+ | 支持 | 支持 | Chrome / Edge | 主要目标 |
| Linux (glibc 2.28+) | 支持 | 支持 | Chrome / Edge | 主要目标 |
| macOS | 待定 | 待定 | 待定 | 未来考虑 |

macOS 的延迟支持原因：Native Messaging Host 在 macOS 上需要代码签名（`codesign`），且 Safari 扩展发布需 Apple Developer Program 会员资格。代码结构设计保持跨平台兼容性（使用 `dirs` crate 处理路径），后续加入 macOS 时不需架构级重构。

### 1.3 设计原则

| 原则 | 说明 |
|------|------|
| 零信任存储 | Vault 文件在静止状态下始终加密，明文数据只在内存中短暂存在 |
| 最小权限 | 各组件只持有完成其职责所需的最小权限和数据 |
| 本地优先 | 无网络依赖，无云端服务，用户完全掌控数据 |
| 无 root 安装 | 全程 user-space 安装，不修改系统级配置 |
| 防御性设计 | 密钥材料用后即清零，UI 显示超时自动锁定 |

### 1.4 威胁模型边界

**在保护范围内：**
- 静止时的 Vault 文件被读取（磁盘丢失、云同步服务被入侵）
- 非授权进程通过 Native Messaging 发送伪造请求

**不在保护范围内：**
- 用户设备已被 root/admin 级恶意软件控制
- 用户主密码本身的强度
- 操作系统级内存 dump

---

## 2. 项目结构

### 2.1 Rust Workspace 布局

```
wpw/
├── Cargo.toml                         # workspace manifest
├── Cargo.lock
├── DESIGN.md
│
├── crates/
│   ├── wpw-core/                      # 核心库（无 I/O 副作用）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── crypto/
│   │       │   ├── kdf.rs             # Argon2id 封装
│   │       │   ├── cipher.rs          # AES-256-GCM 封装
│   │       │   ├── key.rs             # SecretKey 类型，ZeroizeOnDrop
│   │       │   └── mod.rs
│   │       ├── vault/
│   │       │   ├── format.rs          # 文件格式定义
│   │       │   ├── entry.rs           # Entry 数据结构
│   │       │   ├── header.rs          # VaultHeader 解析
│   │       │   └── mod.rs
│   │       ├── generator/
│   │       │   ├── password.rs
│   │       │   ├── passphrase.rs
│   │       │   └── mod.rs
│   │       └── totp.rs
│   │
│   ├── wpw-cli/                       # CLI 二进制
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── init.rs
│   │       │   ├── add.rs
│   │       │   ├── get.rs
│   │       │   ├── list.rs
│   │       │   ├── edit.rs
│   │       │   ├── delete.rs
│   │       │   ├── generate.rs
│   │       │   ├── lock.rs
│   │       │   ├── unlock.rs
│   │       │   ├── export.rs
│   │       │   ├── import.rs
│   │       │   └── mod.rs
│   │       ├── session.rs
│   │       ├── tty.rs
│   │       └── clipboard.rs
│   │
│   └── wpw-host/                      # Native Messaging Host 二进制
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── protocol.rs
│           ├── handler.rs
│           ├── session.rs
│           └── allowed_origins.rs
│
├── extension/                         # 浏览器扩展
│   ├── manifest.json
│   ├── background/
│   │   └── service-worker.js
│   ├── popup/
│   │   ├── popup.html
│   │   ├── popup.js
│   │   └── popup.css
│   ├── content/
│   │   └── autofill.js
│   └── icons/
│       ├── 16.png
│       ├── 48.png
│       └── 128.png
│
├── install/
│   ├── install-windows.ps1
│   ├── install-linux.sh
│   ├── com.wpw.host.json.template
│   └── registry-template.reg
│
└── assets/
    └── wordlist/
        └── eff-large.txt
```

### 2.2 Crate 依赖关系

```
wpw-cli  ──┐
           ├──→ wpw-core
wpw-host ──┘
```

`wpw-core` 不依赖任何 CLI 或网络 I/O 库，保证纯逻辑可测性。

### 2.3 编译产物

| 二进制 | Windows | Linux |
|--------|---------|-------|
| CLI 工具 | `wpw.exe` | `wpw` |
| NM Host | `wpw-host.exe` | `wpw-host` |

两个二进制均以 `release` profile 编译，启用 LTO 以减小体积。

---

## 3. 加密方案

### 3.1 算法选型总览

| 组件 | 算法 | 理由 |
|------|------|------|
| KDF | Argon2id | 抵抗 GPU/ASIC 暴力破解；id 变体同时防旁道和时间攻击 |
| 对称加密 | AES-256-GCM | 硬件加速（AES-NI）；AEAD，防止密文篡改 |
| 随机数源 | OS CSPRNG | Windows `BCryptGenRandom`，Linux `/dev/urandom` |
| TOTP | HMAC-SHA1 | RFC 6238 标准要求 |

### 3.2 主密码处理（KDF）

**算法：Argon2id**

参数存储于 Vault Header，支持未来升级：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `m_cost` | 65536 KiB（64 MiB） | 内存用量 |
| `t_cost` | 3 | 迭代次数 |
| `p_cost` | 4 | 并行度 |
| `output_len` | 64 字节 | 派生两个 32 字节密钥 |
| `salt` | 32 字节随机值 | 每次 `wpw init` 生成 |

**双密钥模型：**

```
KDF Output (64 bytes)
├── bytes[0..32]  → Encryption Key（当前版本使用）
└── bytes[32..64] → HMAC Key（保留，供未来扩展）
```

### 3.3 数据加密（AES-256-GCM）

**加密粒度：整个 Vault Payload 作为一个整体加密。**

不对每条 Entry 单独加密，理由：个人用途体积有限，整体加密写入逻辑简单，nonce 管理无需复杂协调。

**已知限制：**

- 条目数超过 ~1,000 时，每次写入需重新加密整个 Payload，Argon2id + AES-256-GCM 操作耗时可能超过 1 秒，对频繁编辑的交互体验有影响。
- 整体加密意味着任何条目的修改都需要解密全部数据、修改后重新加密整个 Vault，无法利用增量写入优化。

**未来迁移路径：**

若后续版本需支持逐条加密（per-entry encryption），可在 `VaultData.version` 中引入新模式标志。逐条加密方案：每个 Entry 单独 AES-256-GCM 加密（各自独立 nonce），条目元数据（id、title、url）保留明文索引以支持列表和搜索。此模式下每条 entry 拥有独立密钥（从主 encryption_key 通过 HKDF 派生），不设搜索辅助结构，以实现条目级别的加密隔离。

**Nonce 管理：**
- 长度：12 字节（96-bit）
- 来源：每次写入 Vault 时从 OS CSPRNG 生成全新 nonce
- 存储：明文存于 Header
- 绝不复用：每次保存都生成新 nonce，旧文件原子替换

**AAD（Additional Authenticated Data）：**

将 Header 的不变字段（magic bytes + version + salt）作为 AAD，确保 Header 字段也受完整性保护，防止攻击者篡改版本号或 salt。

**主密码正确性验证：**

不单独存储密码哈希。解密时若主密码错误，AES-GCM Authentication Tag 校验失败即报错。好处：不存在"密码验证 Oracle"，攻击者无法通过该接口进行离线验证。

### 3.4 内存中密钥的生命周期

```
解锁 → 派生密钥存入实现了 ZeroizeOnDrop 的结构体
     ↓
操作期间密钥驻留内存
     ↓
锁定 / 超时 / 进程退出 → 自动调用 zeroize() 清零
```

实现要点：
- 使用 `zeroize` crate 的 `Zeroize` + `ZeroizeOnDrop` trait
- 主密码字符串在 KDF 计算完成后立即清零
- 不将密钥材料写入日志、错误消息或 panic 输出
- 避免对敏感字符串调用 `.clone()`

---

## 4. 存储格式设计

### 4.1 Vault 文件位置

| 平台 | 默认路径 |
|------|---------|
| Windows | `%USERPROFILE%\Documents\wpw\vault.wpw` |
| Linux | `~/.local/share/wpw/vault.wpw` |

通过环境变量 `WPW_VAULT_PATH` 或 CLI flag `--vault` 可覆盖路径。

### 4.2 加密文件二进制格式

```
┌──────────────────────────────────────────────────────────┐
│                   VAULT FILE FORMAT                      │
├────────────────────────────────────────────────────────  │
│  HEADER（明文，部分字段作为 GCM AAD）                    │
│                                                          │
│  [0..4]    Magic bytes: 0x57 0x50 0x57 0x00 ("WPW\0")  │
│  [4..6]    Format version: u16 LE（当前: 0x0001）        │
│  [6..10]   Header length: u32 LE                        │
│  [10..14]  Payload length: u32 LE                       │
│  [14..46]  KDF salt: 32 bytes random                    │
│  [46..47]  Argon2id m_cost exponent: u8（实际=2^n KiB） │
│  [47..48]  Argon2id t_cost: u8                          │
│  [48..49]  Argon2id p_cost: u8                          │
│  [49..61]  GCM Nonce: 12 bytes random                   │
│  [61..79]  Reserved: 18 bytes（zero-filled）             │
├────────────────────────────────────────────────────────  │
│  PAYLOAD（加密）                                         │
│                                                          │
│  [H..H+16] GCM Authentication Tag: 16 bytes            │
│  [H+16..]  Ciphertext: AES-256-GCM encrypted            │
│            明文为 MessagePack 序列化的 VaultData         │
└──────────────────────────────────────────────────────────┘
```

Header length 字段允许未来扩展 Header 而不破坏向后兼容性。Auth Tag 置于 Payload 开头，方便解析时提取。

### 4.3 明文 Payload 结构（MessagePack）

解密后的 Payload 使用 MessagePack 序列化（相比 JSON 更紧凑，二进制友好，无注入风险）。

**VaultData：**

```
VaultData {
    version:     u32,        // 数据模型版本，用于迁移
    created_at:  i64,        // Unix timestamp (seconds)
    modified_at: i64,
    entries:     Vec<Entry>,
}
```

**Entry：**

```
Entry {
    id:           String,          // UUID v4
    created_at:   i64,
    modified_at:  i64,

    title:        String,          // 显示名称，如 "GitHub"
    url:          Option<String>,  // 登录页 URL（自动填充匹配用）
    username:     Option<String>,
    password:     Option<String>,  // 明文（Vault 整体加密保护）

    totp_secret:  Option<String>,  // Base32 编码
    totp_issuer:  Option<String>,
    notes:        Option<String>,
    tags:         Vec<String>,
    custom_fields: Vec<CustomField>,
    password_history: Vec<PasswordHistoryEntry>, // 最多保留最近 10 条
}

CustomField {
    label:  String,
    value:  String,
    hidden: bool,    // true 表示在 UI 中遮掩显示
}

PasswordHistoryEntry {
    password:   String,
    changed_at: i64,
}
```

### 4.4 URL 匹配策略

自动填充时，通过以下层级规则将当前页面 URL 与 Entry.url 匹配：

**第一层：域名匹配**

1. **精确域名匹配**：提取当前页面的 eTLD+1（如 `github.com`），与 Entry URL 的 eTLD+1 对比
2. **子域名兼容**：`login.github.com` 匹配 `github.com` 的条目，反之亦然（若 Entry 记录为 `login.github.com`，也匹配 `github.com` 的页面）

**第二层：路径前缀匹配**

若多个 Entry 同属一个域名，则比较当前页面 URL 的 path 前缀与 Entry URL 的 path：
- `https://company.com/admin/` 优先匹配 `admin` 相关条目
- `/admin/settings` 匹配 path 为 `/admin` 的条目（前缀匹配）
- path 匹配度按共享前缀长度降序排列

**第三层：排序规则**

多个候选匹配时的呈现顺序（从高到低）：
1. 精确子域名 + path 前缀最长匹配
2. 精确子域名匹配，无 path 匹配
3. eTLD+1 匹配 + path 前缀匹配
4. eTLD+1 匹配，无 path 匹配
5. 无 URL 的条目（不出现在自动填充候选中，仅手动查看）

**特殊处理：**

- **localhost**：忽略端口号差异，`localhost:3000`、`localhost:8080` 均匹配 `localhost` 条目。若 Entry URL 指定了端口，则精确端口匹配优先。
- **非标准端口**：`example.com:8443` 视为与 `example.com` 不同的匹配目标，不自动降级为 eTLD+1 匹配，需精确端口匹配。
- **IP 地址**：不支持 eTLD+1 提取，使用完整 IP 字符串匹配。

### 4.5 原子写入与备份机制

每次写入 Vault 前：

1. 若当前文件存在，重命名为 `vault.wpw.bak`（覆盖上一个备份）
2. 将新数据写入临时文件 `vault.wpw.tmp`
3. 原子重命名 `vault.wpw.tmp` → `vault.wpw`

跨平台原子重命名：Windows 使用 `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`，Linux 使用 `rename(2)`。

**写时校验：** 写入前读取现有文件的 magic bytes，确认是合法 Vault 文件，防止误操作非 Vault 文件路径。

**版本号单调递增：** `VaultData.version` 在每次写入时递增，读取时若版本号异常则警告（可能读到旧的云同步版本）。

---

## 5. CLI 命令设计

### 5.1 命令结构

CLI 使用 `clap` crate 构建，采用子命令风格：

```
wpw <SUBCOMMAND> [OPTIONS]
```

### 5.2 完整命令列表

**初始化与配置：**

```
wpw init [--vault <PATH>]
    初始化新 Vault，提示输入并确认主密码，生成随机 salt

wpw config set <KEY> <VALUE>
wpw config get <KEY>
    管理配置项（kdf.m_cost / kdf.t_cost / session.timeout 等）

**配置文件规范：**

| 属性 | 值 |
|------|-----|
| 路径 | Windows：`%APPDATA%\wpw\config.toml`；Linux：`~/.config/wpw/config.toml`（遵循 XDG 规范） |
| 格式 | TOML（人类可读，支持注释，Rust `toml` crate 解析） |
| 权限 | 文件 `0600`，目录 `0700`（Linux）；Windows ACL 仅当前用户完全控制 |
| 编码 | UTF-8 |

**配置项分类：**

| 分类 | 示例 key | 敏感 |
|------|---------|------|
| KDF 参数 | `kdf.m_cost`, `kdf.t_cost`, `kdf.p_cost` | 否 |
| 会话设置 | `session.timeout`, `session.max_idle` | 否 |
| TOTP 设置 | `totp.offset` | 否 |
| UI 设置 | `ui.default_format`, `ui.no_color` | 否 |
| Vault 路径 | `vault.path` | 否 |

注意：配置文件不存储密钥材料（主密码、加密密钥、session_key），这些通过 Vault 文件或会话文件独立管理。
```

**会话管理：**

```
wpw unlock [--timeout <SECONDS>] [--vault <PATH>]
    解锁 Vault，将密钥缓存在会话中（默认超时 300 秒）

wpw lock
    主动锁定：清除会话令牌

wpw status
    显示当前 Vault 路径、锁定状态、条目数量
```

**条目管理：**

```
wpw add [--title <TITLE>] [--url <URL>] [--username <USER>]
        [--password <PASS>] [--generate] [--notes <TEXT>]
        [--tag <TAG>...] [--totp <SECRET>]

wpw get <ID|TITLE> [--field <FIELD>] [--copy] [--show]
    默认不显示密码，仅显示元数据
    --copy: 复制到剪贴板（30 秒后自动清除）
    --show: 在终端明文显示密码

wpw list [--tag <TAG>] [--url <URL>] [--format <table|json|csv>]

wpw edit <ID|TITLE> [--title <NEW>] [--url <NEW>] [--username <NEW>]
                    [--password <NEW>] [--generate] [--notes <NEW>]
    密码变更时，旧密码自动推入 password_history

wpw delete <ID|TITLE> [--yes]

wpw history <ID|TITLE>
    显示密码历史记录

wpw restore <ID|TITLE> --at <TIMESTAMP>
    从历史恢复指定时间点的密码
```

**密码生成器：**

```
wpw generate [OPTIONS]
    独立运行，不依赖 Vault 是否解锁

    --length <N>           密码长度（默认 20）
    --upper / --no-upper   包含大写字母（默认开）
    --lower / --no-lower   包含小写字母（默认开）
    --digits / --no-digits 包含数字（默认开）
    --symbols <CHARS>      允许的符号字符集（默认 !@#$%^&*）
    --no-symbols
    --exclude <CHARS>      排除指定字符（如 0O1lI 防视觉混淆）
    --count <N>            生成 N 个候选密码

    --passphrase           切换为密语模式（EFF 词表）
    --words <N>            词数（默认 5）
    --separator <CHAR>     分隔符（默认 -）
    --capitalize           首词首字母大写
```

**TOTP：**

```
wpw totp <ID|TITLE> [--copy]
    实时计算并显示当前 TOTP 码（含剩余有效秒数）
```

**时间窗口策略：**

TOTP 验证码生成依赖系统时间与服务器时间的同步。实现策略：

- **步进容忍**：除当前 30 秒窗口外，同时计算 ±1 步进（前后各一个窗口），以容忍客户端与服务器之间最多 30 秒的时钟偏差。
- **时间偏差检测**：首次运行时记录本地时间与 NTP 时间的差异（参考 `time.is` 或系统 NTP 服务），若偏差超过 60 秒则输出警告。
- **手动偏移配置**：通过 `config set totp.offset <SECONDS>` 设置固定偏移量（正值为提前，负值为延后），适合已知固定偏差的场景。
- 偏移量存储在本地配置文件中，不同步到 Vault。

**导入导出：**

```
wpw export [--format <json|csv|bitwarden>] [--output <FILE>]
    json: 导出明文 JSON（需二次确认）
    bitwarden: 兼容 Bitwarden 导出格式

wpw import [--format <json|csv|bitwarden>] <FILE>
    支持从 Bitwarden、1Password、KeePass CSV 导入
    遇到重复 URL+用户名时提示用户选择合并或跳过
```

**全局 Flag：**

```
--vault <PATH>     覆盖默认 Vault 路径
--config <PATH>    覆盖默认配置文件路径
--no-color         禁用颜色输出
--quiet            只输出结果
--json             以 JSON 格式输出（适用于脚本调用）
```

### 5.3 会话管理机制

**问题：** CLI 每次调用都是独立进程，主密码不能要求每次输入。

**方案：基于文件的会话密钥机制**

不依赖环境变量（环境变量可被子进程继承，也可能被其他进程通过 `/proc/<pid>/environ` 读取），session_key 和加密后的 encryption_key 均存入仅当前用户可读的文件中。

```
unlock 流程：
1. 用户输入主密码，验证 Vault 可解密
2. 生成随机 session_key（32 字节，OS CSPRNG）
3. 用 session_key 加密 encryption_key（AES-256-GCM，随机 nonce）
4. 将加密后的 encryption_key（ciphertext + nonce + tag）写入 session 数据文件
5. 将 session_key 写入独立文件，设权限为仅当前用户可读
6. 记录 unlock_time，用于超时检查

后续命令：
1. 读取 session_key 文件
2. 读取 session 数据文件
3. 解密得到 encryption_key
4. 检查是否超时，超时则拒绝并提示重新解锁

lock 流程：
1. 删除 session 数据文件
2. 删除 session_key 文件
```

**会话文件路径：**

| 平台 | 目录 | session_key 文件 | session 数据文件 |
|------|------|-----------------|-----------------|
| Windows | `%LOCALAPPDATA%\wpw\session\` | `session.key` | `session.dat` |
| Linux | `~/.local/share/wpw/session/` | `session.key` | `session.dat` |

**权限要求：**
- Linux：目录权限 `0700`，文件权限 `0600`
- Windows：目录和文件 ACL 设为仅当前用户完全控制（使用 `SetNamedSecurityInfoW`）

**风险说明：**
- 若攻击者获得当前用户权限，可读取 session_key 和数据文件，解密得到 encryption_key。此威胁不在本项目威胁模型内（参见 §1.4）。
- 若操作系统临时目录 `/tmp` 被用于存放会话文件，需防范符号链接攻击，因此使用用户专有目录而非全局 `/tmp`。
- session_key 与 session 数据文件分存两个独立文件，单一文件泄露不构成解密能力。
- 超时检查仅依赖文件 mtime，不做实时倒计时（独立进程之间无法维护精确计时器）。
---

## 6. Native Messaging 协议

### 6.1 基础机制

Chrome/Edge 的 Native Messaging 使用 stdin/stdout 通信：
- 消息格式：4 字节 little-endian 长度前缀 + UTF-8 JSON 正文
- 单次消息上限：1 MB
- Host 进程由浏览器按需启动，通信结束后退出

### 6.2 消息 JSON Schema

**基础信封（Envelope）：**

```json
{
  "id":      "string (UUID v4，请求/响应配对用)",
  "type":    "string (消息类型)",
  "payload": "object (类型相关字段)"
}
```

**请求类型：**

```json
// 查询锁定状态
{ "id": "...", "type": "status" }

// 解锁
{
  "id": "...", "type": "unlock",
  "payload": { "master_password": "string" }
}

// 主动锁定
{ "id": "...", "type": "lock" }

// 查询匹配当前 URL 的条目
{
  "id": "...", "type": "query",
  "payload": { "url": "https://github.com/login" }
}

// 获取指定条目完整数据（含密码）
{
  "id": "...", "type": "get_entry",
  "payload": { "entry_id": "uuid-string" }
}

// 获取 TOTP 码
{
  "id": "...", "type": "get_totp",
  "payload": { "entry_id": "uuid-string" }
}

// 新增条目（来自扩展的"保存密码"提示）
{
  "id": "...", "type": "add_entry",
  "payload": {
    "title": "string", "url": "string",
    "username": "string", "password": "string"
  }
}
```

**响应类型：**

```json
// 成功响应
{
  "id": "对应请求的 id", "type": "response",
  "success": true,
  "payload": { ... }
}

// 错误响应
{
  "id": "对应请求的 id", "type": "response",
  "success": false,
  "error": { "code": "string", "message": "string" }
}

// Host 主动推送事件（无对应请求）
{
  "id": null, "type": "event",
  "payload": { "event": "locked" | "unlocked" | "vault_changed" }
}
```

**错误码枚举：**

| code | 含义 |
|------|------|
| `vault_locked` | Vault 未解锁 |
| `wrong_password` | 主密码错误 |
| `vault_not_found` | Vault 文件不存在 |
| `entry_not_found` | 指定 entry_id 不存在 |
| `totp_not_configured` | 该条目未配置 TOTP |
| `permission_denied` | 来源扩展 ID 不在白名单 |
| `io_error` | 文件系统操作失败 |
| `internal_error` | 内部错误 |

**错误处理策略：**

**消息脱敏规则：**

错误响应中的 `message` 字段遵循最小信息披露原则，避免泄露系统内部状态：
- 文件路径、Vault 内容片段、密钥材料绝不出现在错误消息中
- 通用错误（`io_error`、`internal_error`）仅返回固定消息，详细信息写入本地日志
- `wrong_password` 错误不区分"密码错误"与"文件损坏"，防止侧信道信息泄露

**Panic Hook：**

Rust 侧注册自定义 panic hook：
- panic 发生时，先 zeroize `SessionKey` 和 `EncryptionKey`（防止 panic unwind 路径跳过 Drop），再调用默认行为
- panic 消息不包含敏感数据（使用 `set_hook` 替换 `human_panic` 或 `color-eyre` 的默认输出）
- Native Host panic 时通过 NM 通道返回 `internal_error` 后再退出

**日志级别策略：**

| 级别 | 使用场景 | 包含敏感数据 |
|------|---------|------------|
| ERROR | 不可恢复错误（Vault 损坏、IO 失败） | 否 |
| WARN | 可恢复异常（密码强度弱、时钟偏差） | 否 |
| INFO | 正常操作流（解锁/锁定、条目增删） | 否（仅 UUID） |
| DEBUG | 开发调试信息 | 否（编译期 `#[cfg(debug_assertions)]` 门控） |
| TRACE | 详细诊断 | 否 |

日志输出默认写入 stderr（CLI）或系统日志（Host），不写入文件以避免信息泄露。

### 6.3 敏感字段传输说明

密码字段在 Native Messaging JSON 中以明文字符串传输。

**安全依据：** Native Messaging 的 stdin/stdout 管道由 OS 管理，其他进程无法读取。风险不在于传输层，因此不增加额外加密层（复杂度高而安全收益极小）。

### 6.4 安全验证机制

**双重来源验证：**

1. **manifest 层**：`allowed_origins` 字段由浏览器强制执行，只有白名单扩展能调用 Host
2. **代码层**：Host 启动时从命令行参数提取调用方扩展 ID，与编译期内置白名单对比，不匹配则立即退出

Chrome 启动 Host 时传入扩展 ID：

```
wpw-host chrome-extension://EXTENSION_ID.../
```

内置白名单：
```
ALLOWED_EXTENSION_IDS = [
    "Chrome Web Store 发布后的 ID",
    "Edge Add-ons 发布后的 ID",
    "开发模式 unpacked 扩展的本地 ID（仅 debug build）"
]
```

**防重放：** 每个请求携带 UUID v4 `id`，扩展侧校验响应 `id` 与请求一致。

### 6.5 Host 进程会话状态

Host 进程在内存中维护 `{ locked, encryption_key }` 状态：
- 解锁后 encryption_key 驻留内存，后续请求直接使用
- Host 进程退出（浏览器关闭）时 encryption_key 随进程消亡（ZeroizeOnDrop）
- Chrome 可能在空闲时终止 Native Host，扩展需在每次通信时检查连接是否存活，重连后状态为 locked

---

## 7. 浏览器扩展架构

### 7.1 Manifest V3 关键配置

```json
{
  "manifest_version": 3,
  "name": "WPW Password Manager",
  "version": "1.0.0",
  "permissions": ["nativeMessaging", "activeTab", "scripting", "clipboardWrite"],
  "host_permissions": ["<all_urls>"],
  "background": {
    "service_worker": "background/service-worker.js",
    "type": "module"
  },
  "action": {
    "default_popup": "popup/popup.html"
  }
}
```

`content_scripts` 不在 manifest 中静态声明，而是通过 `scripting.executeScript` 按需注入，避免在无关页面运行内容脚本。

### 7.2 Service Worker 职责

- **与 Native Host 通信**：维护连接，封装请求/响应，管理 5 秒超时
- **状态管理**：缓存 `{ locked, vaultExists, currentTabEntries }` 等 UI 状态（注意 Service Worker 可能随时休眠）
- **与 Popup 通信**：监听 `chrome.runtime.onMessage`，转发响应
- **与 Content Script 通信**：接收"检测到登录表单"消息，决策是否触发填充

**状态恢复机制：**

Service Worker 在 Manifest V3 中可能随时被浏览器休眠（闲置约 30 秒后），需处理状态丢失：

- **启动时**：通过 `chrome.runtime.onStartup` 和 Service Worker 激活事件检查 Native Host 连接状态，重新初始化 `{ locked: true }` 状态
- **缓存策略**：`currentTabEntries` 等易失数据使用内存缓存；跨休眠的持久状态（如 `vaultExists`）通过 `chrome.storage.session` 保存
- **重连逻辑**：每次与 Native Host 通信前检查连接是否存活，若断开则重置为 locked 状态并更新 badge

**连接状态管理：**

```
状态机：disconnected → connecting → connected → (断开) → disconnected
                                     ↓
                                  locked → (unlock成功) → unlocked
```

- 心跳检测：每 30 秒发送 `status` 请求验证连接存活
- 超时处理：响应超时 5 秒后重试一次，仍失败则标记为 disconnected，更新 UI 状态

**Badge 状态指示器：**

| Badge 文字 | 颜色 | 含义 |
|-----------|------|------|
| （无） | — | Vault 未配置或无法连接 |
| `!` | 红色 | Vault 已锁定，需解锁 |
| `N`（数字） | 蓝色 | 已解锁，N 为当前页面匹配条目数 |
| `·` | 灰色 | 已解锁，当前页面无匹配条目 |

### 7.3 Popup UI 交互流程

```
Popup 打开
    ↓
查询 status
    ├─ vault_not_exists → 显示初始化引导（提示运行 wpw init）
    ├─ locked          → 显示主密码输入框
    │                       ↓ 解锁成功 → 进入已解锁状态
    │                       ↓ 失败     → 显示错误，清空输入框
    └─ unlocked        → 查询当前 URL 匹配条目，显示列表
                             ↓ 用户点击条目
                         [填充] → 向 Content Script 发送填充指令
                         [复制用户名] → 写入剪贴板
                         [复制密码]   → 写入剪贴板（30s 后清除）
                         [显示 TOTP]  → 请求 TOTP 码，显示倒计时

底部常驻：[锁定] [设置]
```

Popup 安全规则：
- 主密码输入框使用 `type="password"`
- 密码字段在 DOM 中不以明文渲染
- Popup 关闭时清除内存中的敏感字段引用

### 7.4 Content Script 自动填充实现

**注入时机：** Service Worker 在 `chrome.tabs.onUpdated` 或用户触发时，使用 `scripting.executeScript` 按需注入。

**表单检测逻辑：**

```
1. 扫描页面中的 <input type="password"> 元素
2. 向上遍历 DOM 寻找关联 <form> 或相邻 username 输入框
3. 向 Service Worker 发送 { type: "form_detected", url: location.href }
4. Service Worker 查询匹配条目：
   - 唯一匹配 → 自动填充
   - 多个匹配 → 弹出 Popup 让用户选择

填充执行：
1. 接收 { username, password }
2. 找到目标输入框，设置 .value
3. 触发 input / change / keyup 事件（兼容 React/Vue 响应式绑定）
4. 立即将 password 变量赋值为空字符串
```

安全约束：
- Content Script 不缓存密码，用完即丢
- 与 Service Worker 的通信通过 `chrome.runtime.sendMessage`，不经过页面 DOM
- 不向页面注入任何全局变量

### 7.5 Native Messaging Host 注册

#### Windows（注册表）

```
HKEY_CURRENT_USER\SOFTWARE\Google\Chrome\NativeMessagingHosts\com.wpw.host
→ %LOCALAPPDATA%\wpw\nm-manifest.json

HKEY_CURRENT_USER\SOFTWARE\Microsoft\Edge\NativeMessagingHosts\com.wpw.host
→ %LOCALAPPDATA%\wpw\nm-manifest.json
```

`nm-manifest.json`：

```json
{
  "name": "com.wpw.host",
  "description": "WPW Password Manager Native Host",
  "path": "C:\\Users\\<User>\\AppData\\Local\\wpw\\wpw-host.exe",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://EXTENSION_ID_HERE/"]
}
```

#### Linux（文件）

Chrome：`~/.config/google-chrome/NativeMessagingHosts/com.wpw.host.json`

Edge：`~/.config/microsoft-edge/NativeMessagingHosts/com.wpw.host.json`

---

## 8. 安装与分发

### 8.1 设计目标

- 无需管理员/root 权限
- 单脚本完成全部配置
- 支持覆盖安装升级
- 提供 `--uninstall` 选项

### 8.2 Windows 安装流程（PowerShell）

```
1. 创建目录 %LOCALAPPDATA%\wpw\
2. 复制 wpw.exe、wpw-host.exe
3. 将 %LOCALAPPDATA%\wpw\ 添加到当前用户 PATH
   （修改 HKCU\Environment\Path，不影响系统 Path）
4. 生成 nm-manifest.json（填入实际路径和扩展 ID）
5. 写入注册表（Chrome + Edge）
6. 输出摘要，提示重启终端
```

### 8.3 Linux 安装流程（Shell）

```
1. 创建 ~/.local/bin/、~/.local/share/wpw/
2. 复制二进制文件并 chmod +x
3. 检查 ~/.local/bin 是否在 PATH 中，若不在则追加到 .bashrc/.zshrc
4. 生成 NM manifest 文件
5. 写入 Chrome、Edge 的 NativeMessagingHosts 目录
6. 输出摘要，提示 source ~/.bashrc
```

### 8.4 扩展 ID 的鸡与蛋问题

NM Manifest 中的 `allowed_origins` 需要扩展 ID，而扩展 ID 在发布前未知：

- **开发阶段**：安装脚本接受 `--extension-id <ID>` 参数
- **发布阶段**：将正式 ID 写入安装脚本默认值
- **双 ID 支持**：debug build 额外允许本地 unpacked 扩展 ID

### 8.5 分发方式

- GitHub Releases 发布 Windows（zip）和 Linux（tar.gz）压缩包
- 浏览器扩展发布到 Chrome Web Store 和 Microsoft Edge Add-ons
- 不提供 MSI/deb 包，避免需要管理员权限

---

## 9. 关键安全考量

### 9.1 剪贴板自动清除

**CLI：**
1. 密码写入剪贴板后，记录写入时间和内容
2. 后台线程睡眠 30 秒
3. 若剪贴板内容未变则清空，否则不操作
4. 终端输出提示："密码已复制，将在 30 秒后清除剪贴板"

**浏览器扩展：**
1. `navigator.clipboard.writeText(password)`
2. Service Worker 中设置 `setTimeout(() => navigator.clipboard.writeText(''), 30000)`
3. Popup 重新打开时额外检查并清除（应对 Service Worker 休眠导致计时器失效）

### 9.2 内存中明文密码的清零

**Rust 侧：**
- 所有持有密码字节的结构体实现 `zeroize::ZeroizeOnDrop`
- 使用 `secrecy` crate 的 `SecretString` 包装密码字符串
- 避免对敏感字符串调用 `.clone()`
- 使用 `zeroize` crate 的 volatile write + compiler fence 确保清零不被优化掉

**JavaScript 侧：**
- JS 无法真正控制 GC，只能缩短密码字符串的引用生命周期
- 填充完成后立即 `password = ''` 解除引用
- 不将密码存入 `localStorage`、`sessionStorage` 或 IndexedDB

### 9.3 Native Messaging Origin 验证

**双重验证：**
1. `allowed_origins`（manifest）：浏览器强制执行，防止外部扩展调用
2. 命令行参数验证（Host 代码内置白名单）：纵深防御，防止 manifest 被恶意修改

### 9.4 防止 Vault 文件意外覆盖

- 写入前读取 magic bytes，确认是合法 Vault 文件
- 云同步冲突文件检测：`wpw status` 时检查并警告
- VaultData.version 单调递增，读取时若版本号异常则警告

### 9.5 主密码强度提示

`wpw init` 时对主密码进行强度评估（zxcvbn 算法），强度不足时显示警告，但不强制拒绝。

### 9.6 防止进程间信息泄露

- 不在进程标题或环境变量中包含密钥材料
- CLI **不支持**通过 `--password <PASS>` 在命令行直接传入密码（`ps aux` 会暴露），强制从 stdin 读取或从已解锁会话获取

### 9.7 密码历史记录管理

每次通过 `wpw edit` 或 `wpw add` 修改密码时，旧密码自动推入 `password_history` 数组：

- **保留上限**：每个 Entry 最多保留最近 10 条历史记录（FIFO，超限时移除最早的记录）
- **存储加密**：历史密码以明文形式存储于加密的 Vault Payload 内，安全性与当前密码一致
- **查看与恢复**：`wpw history <ID>` 显示变更时间列表；`wpw restore <ID> --at <TIMESTAMP>` 将指定历史密码恢复为当前密码（当前密码同步推入历史）
- **导出隔离**：`wpw export` 默认不包含 `password_history` 字段，需 `--include-history` flag 明确确认后导出
- 密码历史仅在条目密码字段变更时记录，手动编辑其他字段（如 title、url）不触发历史记录

---

## 10. 依赖选型

### 10.1 Rust Crates

| crate | 用途 |
|-------|------|
| `argon2` | Argon2id KDF（RustCrypto，纯 Rust，无 C 依赖） |
| `aes-gcm` | AES-256-GCM（RustCrypto，经过审计） |
| `rand` / `getrandom` | CSPRNG |
| `zeroize` | 内存清零（防优化器消除） |
| `secrecy` | SecretString / SecretBox（基于 zeroize） |
| `serde` + `rmp-serde` | MessagePack 序列化 |
| `clap` | CLI 参数解析（derive macro） |
| `toml` | 配置文件解析 |
| `totp-rs` | TOTP 计算（RFC 6238） |
| `uuid` | Entry ID 生成（v4） |
| `time` | 时间戳、超时计算（比 `chrono` 更轻量，无 historical 时区数据依赖） |
| `dirs` | 跨平台标准目录路径（XDG / Windows Known Folders） |
| `arboard` | 剪贴板操作（跨平台） |
| `rpassword` | 终端隐藏密码输入（跨平台） |
| `anyhow` | CLI 快速错误传播 |
| `thiserror` | core 库错误类型定义 |

### 10.2 JavaScript（浏览器扩展）

**零第三方依赖**（原则）：减少供应链攻击面，扩展逻辑简单，原生 API 足够。

可选：`tldts`（eTLD+1 提取，用于 URL 域名匹配）。

扩展代码使用 ES2022 模块语法，不使用打包工具（Webpack/Rollup），保持代码可审计性。

### 10.3 版本锁定策略

- `Cargo.lock` 提交到 Git，确保构建可重现
- 不使用 `*` 或过于宽松的 semver 范围
- 依赖升级通过手动 `cargo update` + 审查 changelog 执行

---

## 附录：关键数据流

### 解锁并自动填充完整流程

```
用户访问 https://github.com/login
        ↓
Service Worker 检测到 Tab 更新
        ↓
向 wpw-host 发送 query 请求
        ↓
wpw-host 检查会话状态
        ├─ locked   → 返回 vault_locked 错误，badge 提示用户解锁
        └─ unlocked → 解密 Vault，过滤匹配条目
                ↓
        返回 [{ id, title, username }]（不含密码）
                ↓
        Service Worker 缓存结果，badge 显示条目数
                ↓
用户点击扩展图标，Popup 显示匹配条目列表
        ↓
用户点击 [填充]
        ↓
Popup → Service Worker → wpw-host: get_entry 请求
        ↓
wpw-host 返回 { username, password }
        ↓
Service Worker 通过 scripting.executeScript 注入 autofill.js
        ↓
autofill.js 填充表单，触发事件，清空本地密码引用
        ↓
Popup 关闭
```
