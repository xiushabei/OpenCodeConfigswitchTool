# OMO 配置切换器

[简体中文](README.md) | [English](README.en.md)

Windows Tauri 2 桌面应用，分别管理 OpenCode 和 oh-my-openagent 的 JSON 配置预设。界面采用 Material Design 3 配色、组件状态和 Material Symbols 图标，支持浅色、深色和跟随系统。

## 下载运行

从 [GitHub Releases](https://github.com/xiushabei/OpenCodeConfigswitchTool/releases/latest) 下载 Windows x64 便携 ZIP，解压到可写目录，双击 `omo-config-switcher.exe`。请将包内的 `switcher.settings.json` 保留在 EXE 同目录。需要 Microsoft Edge WebView2 Runtime，Windows 11 通常已安装；首次启动前可编辑设置文件里的目标路径。

发布版不会弹出命令行窗口，界面禁用浏览器右键菜单，复制/粘贴键盘快捷键仍可使用。当前应用界面为中文，提供中英文使用说明。

操作音效可在“设置 → 操作音效”中开关，开关状态在本机保存。点击、成功和失败使用不同的本地音效，编辑文本及启动时不发声。音效素材来自 [Kenney Interface Sounds](https://kenney.nl/assets/interface-sounds)，使用 CC0 许可并随程序离线打包。

## 启动

需要 Node.js 或 Bun、Rust MSVC 工具链、Microsoft C++ Build Tools（“使用 C++ 的桌面开发”）和 WebView2。Windows 11 通常已有 WebView2。Windows 构建要求以 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/) 为准。

```powershell
bun install
Copy-Item switcher.settings.example.json switcher.settings.json
bun run tauri dev
```

前端检查与生产构建：

```powershell
bun run check
bun run build
bun run tauri build
```

Rust 文件行为测试：`cargo test --manifest-path src-tauri/Cargo.toml`。当前版本已通过 89 项 Rust 测试、10 项 Bun 测试、严格 Clippy、前端构建与真实桌面操作验证。

单独运行 `bun run dev` 只提供浏览器界面预览；配置文件操作需要 Tauri 桌面环境。

## 数据与目标路径

项目根目录中的 `switcher.settings.json` 保存两个目标路径和主题。默认目标如下，`~` 在运行时解析为当前 Windows 用户目录：

```json
{
  "targets": {
    "opencode": "~/.config/opencode/opencode.json",
    "omo": "~/.config/opencode/oh-my-openagent.json"
  },
  "theme": "system"
}
```

在应用的设置中修改目标。相对路径相对于本项目根目录；代码没有硬编码本机绝对路径。设置、预设、备份、导出和 WebView 数据留在项目内，仅点击应用预设时写入所选的外部目标。

程序启动时查找包含 `switcher.settings.json` 的项目根目录。请保留此文件及项目可写目录；移动可执行文件时，应与项目设置和数据一起移动，或从项目目录启动。

## 添加和切换预设

1. 选择 OpenCode 或 oh-my-openagent 目标，点击新建，填写名称、描述和 JSON 对象。
2. 保存预设。也可以导入已有 `.json` / `.jsonc` 配置，或复制已有预设后修改；名称和描述可直接编辑。
3. 打开差异视图，核对预设与目标文件当前内容。
4. 点击应用预设。程序重新校验 JSON，先将现有目标的原始内容备份至 `./backups/`，再写入选定配置。

导出文件位于项目 `./exports/`，可再次导入。预设保存在 `./data/`。删除预设不会删除实际目标文件。备份和导出包含配置原文，若配置中有密钥，这些文件也会包含密钥。

## 启动导入与还原

只有首次启动时会自动读取设置指定的 OpenCode、OMO 配置，新增“当前 OpenCode”“当前 OMO”预设。完成状态保存在项目预设数据中，后续启动即使目标内容改变也不再自动导入；旧版已有预设数据视为已完成首次导入。程序每次启动仍保存两个目标的原始内容，供右上角“还原至原配置”使用。刷新、切换预设、手动导入或修改目标设置不会重置本次启动的还原基准。

“导入配置文件”可添加本地配置，也可以将配置文件拖入窗口。原始配置按文件名识别 OpenCode / OMO，无法识别时使用当前选中的类别；导出的预设文件沿用文件中记录的类别。

“导入当前配置”每次读取当前设置所指向的两个配置文件，并追加新的预设，即使内容相同也不会覆盖先前导入。重名时添加 `(2)`、`(3)` 等后缀。旧自动预设的“启动导入 OpenCode / OMO”名称会转换为“当前 OpenCode / OMO”；用户自行重命名的预设不受影响。

点击“还原至原配置”后会先弹出二次确认；只有再次点击弹窗中的“还原”才执行，取消不会修改目标文件或生成备份。有未保存的编辑时，还会先询问是否放弃编辑。

确认还原后会先备份当前目标，再恢复本次启动时的原文（包括注释、换行）；本次启动时不存在的目标会恢复为不存在。还原对应的是启动时的路径，修改设置不会把旧内容写到新路径。退出重开程序后，还原基准变为这次启动时的状态。还原后需重启 OpenCode 读取配置。

## 验证切换生效

建议先将目标设置为项目内的测试文件，完成一次切换，再改回实际配置路径。

应用成功后，检查高亮激活标记、目标文件内容和新生成的带时间戳备份；差异视图应不再显示已保存预设与目标之间的内容差异。重新打开应用，激活状态应保持与目标实际内容一致；若其他程序修改目标，刷新后会重新判断。

写入成功表示配置文件已更新。正在运行的 OpenCode 会话可能仍使用启动时加载的配置；退出并重新启动 OpenCode，再确认目标模型或其他已修改设置。不要用当前会话的行为判断文件是否写入成功。

程序仅检查 JSON 对象的语法，不会臆测或改写 OpenCode / OMO 字段。字段含义与版本兼容性请参考 [OpenCode 配置文档](https://opencode.ai/docs/config/) 和 [oh-my-openagent 官方仓库](https://github.com/code-yeongyu/oh-my-openagent)。

## 模型列表与 OMO 模型选择

“模型列表”读取本机 OpenCode CLI 的 `opencode models` 输出，因此列表包含 OpenCode 合并后的可用提供商与模型，而不是直接下载 models.dev 目录。需要先安装 OpenCode CLI 并使其可从 PATH 运行。“刷新模型缓存”显式执行 `opencode models --refresh`，可能需要联网。

读取时以切换器项目根目录为工作目录，并通过 `OPENCODE_CONFIG` 指向设置中的 OpenCode 目标文件；OpenCode 自身的全局配置、项目配置、凭据和插件仍按其规则参与合并。模型列表对应当前磁盘配置，不会为了预览自动应用未保存的预设。可按提供商和模型 ID 筛选，界面显示读取目录。

OMO 预设提供“模型选择”和原始 JSON 编辑方式。模型选择只修改 `agents.<名称>.model` 或 `categories.<名称>.model`，保留其他字段；原始 JSON 修改后可切回模型选择查看。当前模型不在目录中时保留原值，不会自动替换。添加条目时填写实际代理或类别名称；字段规则以所用 OpenCode 版 OMO 的文档为准。

## 聊天记录

聊天记录界面从 `opencode.db` 只读解析会话列表与消息，兼容已识别的两代会话表结构。优先读取 `OPENCODE_DB`、`opencode db path` 和默认用户目录；固定位置都未找到时，才进行一次全盘搜索，也可填写数据库路径。搜索会跳过无权限和重解析目录。

数据库区域显示实际文件体积，存在 WAL 时单独列出其体积。会话共用 SQLite 文件，没有独立的会话文件；列表的“内容体积”为会话消息与片段的字节量，支持按内容体积和时间排序，不代表删除后可直接回收的磁盘空间。阅读视图区分正文、思考和工具记录：用户与助手正文默认完整显示；工具记录默认收起为简短摘要，思考记录较短时直接展开，点击摘要或使用键盘均可展开、收回完整内容。

阅读接口返回会话的全部消息与片段，前端不再做二次裁剪。完整读取的内存占用随会话体积增长，超大会话的读取时间取决于数据库大小。仅当后端报告某段数据无法完整解析时，界面标注“解析不完整”；正常折叠的摘要不会被误标为截断。

删除会话前必须完全退出 OpenCode。程序检测到相关进程时会阻止删除；确认框列出本次删除的会话及关联子会话，二次确认后先创建 SQLite 一致性备份，再执行删除事务。无法确认表结构或级联关系时拒绝删除，不自动清理关系不明的 blob。备份保存在切换器的 `backups/` 中。开发验证只使用隔离测试库，不修改正在运行的 OpenCode 会话数据库。

## 许可证

[Apache License 2.0](LICENSE)。
