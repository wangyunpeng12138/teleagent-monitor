---
AIGC:
  ContentProducer: '001191110102MAD55U9H0F10002'
  ContentPropagator: '001191110102MAD55U9H0F10002'
  Label: '1'
  ProduceID: '223deaf5-91d7-4181-96db-a193cb099884'
  PropagateID: '223deaf5-91d7-4181-96db-a193cb099884'
  ReservedCode1: '9e2f8787-3447-4acf-81c5-50b629af549d'
  ReservedCode2: '9e2f8787-3447-4acf-81c5-50b629af549d'
---

# TeleAgent Monitor

TeleAgent 任务状态悬浮窗监控工具。以置顶悬浮窗形式实时展示 TeleAgent 的会话任务进度、定时任务执行状态，支持系统托盘常驻、配置外部化、多 PC 移植。

## 功能概览

- **加速球形态**：默认以桌面加速球形式展示（可吸附屏幕边缘），上格为刷新按钮、下格为红绿灯状态球（蓝=运行中/黄=需介入/绿=空闲），点击红绿灯展开面板，面板 🔵 按钮可最小化回加速球
- **会话监控**：实时展示最近会话的任务列表（Todo）与执行进度，自动折叠/展开
- **四态状态标识**：
  - 🔵 运行中（蓝色脉动圆点）
  - 🟡 需介入（黄色脉动圆点）
  - 🟢 已完成（绿色圆点）
  - 🔴 已终止（红色圆点，用户手动终止后的会话）
- **定时任务面板**：展示定时任务的调度规则与最近执行状态
- **标签过滤**：按工作目录标签快速筛选会话
- **系统托盘**：左键切换面板显示/隐藏，右键菜单（显示面板/设置/隐藏到托盘/退出程序），✕ 按钮隐藏不退出
- **配置外部化**：通过设置面板修改数据目录、轮询间隔、超时阈值等参数，保存后即时生效
- **文件监听**：监听 `session-status.json` 变更，实时触发刷新

## 运行适配软件

| 组件 | 要求 |
|------|------|
| TeleAgent | 已安装并运行过至少一次（需生成 `~/.local/share/TeleAgent` 数据目录） |
| 操作系统 | Windows 10/11 x64 |
| WebView2 | Windows 11 自带；Windows 10 需安装 [Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) |

## 架构设计

```
┌──────────────────────────────────────────────────┐
│                   TeleAgent Monitor               │
│                                                   │
│  ┌──────────────┐      ┌───────────────────────┐ │
│  │   Frontend   │      │       Backend         │ │
│  │  (Vue 3 +    │ IPC  │    (Rust + Tauri 2)   │ │
│  │   Vite +     │◄────►│                       │ │
│  │   TypeScript)│      │  ┌─────┐ ┌─────────┐  │ │
│  │              │      │  │db.rs│ │config.rs│  │ │
│  │ ┌──────────┐ │      │  └─────┘ └─────────┘  │ │
│  │ │App.vue   │ │      │  ┌────────┐ ┌──────┐  │ │
│  │ ├──────────┤ │      │  │commands│ │watcher│  │ │
│  │ │SessionCard│ │      │  │  .rs  │ │ .rs  │  │ │
│  │ ├──────────┤ │      │  └────────┘ └──────┘  │ │
│  │ │Scheduler │ │      │  ┌──────────────┐    │ │
│  │ │Panel.vue │ │      │  │   main.rs    │    │ │
│  │ ├──────────┤ │      │  │ (托盘+窗口)   │    │ │
│  │ │Settings  │ │      │  └──────────────┘    │ │
│  │ │Panel.vue │ │      │                       │ │
│  │ └──────────┘ │      └───────────────────────┘ │
│  └──────────────┘                                 │
│                    ▲                              │
│                    │ 只读（immutable=1）          │
│                    ▼                              │
│  ┌──────────────────────────────────────────────┐│
│  │        TeleAgent 数据目录                     ││
│  │  ~/.local/share/TeleAgent/                    ││
│  │  ├── teleagent.db        (会话/消息/Todo)      ││
│  │  ├── session-status.json (会话状态映射)        ││
│  │  ├── deleted-session-ids.json                  ││
│  │  └── scheduler/scheduler.db (定时任务)         ││
│  └──────────────────────────────────────────────┘│
└──────────────────────────────────────────────────┘
```

### 数据流

1. **定时轮询**：前端按配置间隔调用 `fetch_running_sessions` / `fetch_scheduler_jobs` 命令
2. **文件监听**：`watcher.rs` 监听 `session-status.json` 变更，通过 Tauri 事件通知前端即时刷新
3. **只读访问**：SQLite 使用 `immutable=1` URI 模式打开，完全不加文件锁，不干扰 TeleAgent 运行
4. **状态推导**：后端综合 `session-status.json` 标记、最后消息的 `finish` 字段、消息时间戳推导四态状态

### 状态推导逻辑

| 优先级 | 条件 | 结果 |
|--------|------|------|
| 1 | 最后消息 `finish=stop` | completed（确认结束） |
| 2 | `session-status.json` 标记 `paused` 且在超时阈值内 | needs_human |
| 3 | `session-status.json` 标记 `paused` 但超时 | terminated（手动终止） |
| 4 | `session-status.json` 标记 `running` 且在超时阈值内 | running / needs_human |
| 5 | `session-status.json` 标记 `running` 但超时 | completed（崩溃兜底） |
| 6 | 其他 | completed |

## 运行环境

### 开发环境

| 工具 | 版本 |
|------|------|
| Rust | 稳定版（推荐 1.75+） |
| Node.js | 18+ |
| Tauri CLI 2 | `@tauri-apps/cli@^2` |
| MSVC | Visual Studio Build Tools（C++ 工作负载） |
| WebView2 | WebView2 Runtime（Windows 11 自带） |

### 目录结构

```
teleagent-monitor/
├── src/                        # 前端源码
│   ├── App.vue                 # 主组件（标题栏、布局、标签过滤）
│   ├── main.ts                 # 入口（按窗口 label 挂载 App/SpeedBall）
│   ├── components/
│   │   ├── SpeedBall.vue       # 加速球组件（刷新按钮 + 红绿灯）
│   │   ├── SessionCard.vue     # 会话卡片（折叠/展开、状态圆点、进度摘要）
│   │   ├── SchedulerPanel.vue  # 定时任务面板
│   │   ├── SettingsPanel.vue   # 设置面板（配置外部化 UI）
│   │   └── TodoItem.vue        # 子任务项组件
│   ├── composables/
│   │   ├── types.ts            # TypeScript 类型定义
│   │   ├── useMonitor.ts       # 数据采集 Hook（轮询、配置加载）
│   │   ├── useWindowDrag.ts    # 窗口拖拽（面板）
│   │   ├── useBallSnap.ts      # 加速球吸附边缘
│   │   └── logger.ts           # 前端日志
│   └── ...
├── src-tauri/                  # 后端源码
│   ├── src/
│   │   ├── main.rs             # 入口（Tauri 窗口、托盘、命令注册）
│   │   ├── db.rs               # 数据库读取、状态推导逻辑
│   │   ├── config.rs           # 配置管理（加载/保存/全局实例）
│   │   ├── commands.rs         # Tauri IPC 命令定义
│   │   └── watcher.rs          # 文件监听模块
│   ├── Cargo.toml              # Rust 依赖配置
│   ├── tauri.conf.json         # Tauri 应用配置（窗口、托盘、打包）
│   ├── capabilities/
│   │   └── default.json        # 权限配置
│   └── icons/                  # 应用图标
├── dist/                       # 前端构建产物（自动生成）
├── package.json
├── vite.config.ts
├── tsconfig.json
├── 运行指南.md                  # 详细运行说明
└── README.md                   # 本文件
```

## 编译运行

### 开发模式

```bash
# 1. 安装前端依赖
npm install

# 2. 启动开发模式（自动编译前端 + Rust 热重载）
npm run tauri dev
```

### Release 编译

```bash
# 方式一：标准 Tauri 构建命令
npm run tauri build

# 方式二：Windows 下的编译脚本（需 MSVC 环境）
# 参考项目内的 .temp/tauri-release2.bat
```

编译产物位于：
- `src-tauri/target/release/teleagent-monitor.exe` — 独立可执行文件（约 5-6 MB）
- `src-tauri/target/release/bundle/msi/` — MSI 安装包
- `src-tauri/target/release/bundle/nsis/` — NSIS 安装包

### 运行

直接双击 `teleagent-monitor.exe` 即可运行，无需安装。
首次运行会在 exe 同目录自动生成 `config.json` 配置文件。

## 配置说明

通过悬浮窗标题栏的 ⚙️ 按钮可打开设置面板修改以下配置：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| TeleAgent 数据目录 | `~/.local/share/TeleAgent` | TeleAgent 运行数据所在目录 |
| 会话刷新间隔 | 3000ms | 每隔多久刷新一次会话列表 |
| 定时任务刷新间隔 | 10000ms | 每隔多久刷新一次定时任务 |
| 最大会话显示数 | 15 | 面板中最多显示的会话数量 |
| 会话超时阈值 | 10 分钟 | 超过此时间无新消息的运行中会话判定为结束 |

配置保存在 exe 同目录的 `config.json` 文件中，可直接编辑该文件修改配置。
修改配置后点击保存即可即时生效（轮询间隔自动重启）。

## 移植到其他 PC

1. 将 `teleagent-monitor.exe` 复制到目标 PC
2. 双击运行，自动生成 `config.json`
3. 打开设置面板修改 "TeleAgent 数据目录" 为目标 PC 上的实际路径
4. 保存即可

> 注意：目标 PC 需已安装 WebView2 Runtime（Windows 11 自带）。

## 效果展示

- **加速球**：竖条小窗，上格刷新按钮、下格红绿灯（蓝=运行中/黄=需介入/绿=空闲），点击红绿灯展开面板，拖动后自动吸附屏幕边缘
- **标题栏**：TeleAgent 在线状态 + 运行中/需介入/已终止计数 + 时钟 + 设置/刷新/加速球/关闭按钮
- **会话卡片**：默认折叠显示进度摘要，点击展开查看子任务详情
- **状态圆点**：蓝色脉动（运行中）、黄色脉动（需介入）、绿色（已完成）、红色（已终止）
- **定时任务**：展示任务名、调度规则（工作日/每天/一次性）、最近执行时间与状态
- **系统托盘**：常驻托盘图标，左键切换面板，右键菜单（显示面板/设置/隐藏到托盘/退出）

## 技术栈

- **前端**：Vue 3.5 + TypeScript + Vite 6
- **后端**：Rust + Tauri 2（tray-icon feature）
- **数据库**：SQLite（rusqlite 0.32，immutable 只读模式）
- **文件监听**：notify 7（监听 session-status.json 变更）
- **日志**：env_logger（后端）+ 自定义 logger（前端通过 IPC 写入后端日志）

## License

MIT

> AI生成