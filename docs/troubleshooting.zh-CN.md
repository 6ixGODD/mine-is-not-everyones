# MINE 故障排查

这份文档只处理“已经失败了，而且你不知道如何恢复”的情况。正常安装、开发和发布流程请看[用户指南](user-guide.zh-CN.md)。

## Agent 已安装，但里面看不到 MINE

先看机器级状态：

```sh
mine agent status
```

如果对应 Agent 不是 healthy，重新执行该客户端的 setup：

```sh
mine setup --agents <slug>
```

然后重启 Agent，让它重新加载 Skill 和 MCP 配置。

如果 setup 仍然识别不到客户端，检查运行 `mine setup` 的环境里，客户端可执行文件和配置目录是否真的可见。这里不要一上来用 `mine doctor`：`doctor` 还会检查仓库状态，而 `agent status` 专门回答机器级安装问题。

## `mine init` 因 Design ownership mismatch 拒绝继续

受 MINE 管理的 Design 会在 `docs/design/.mine-design.toml` 记录仓库所有权。如果这个 marker 属于另一个仓库，MINE 不会静默接管。

先看仓库级诊断：

```sh
mine doctor --format json
```

不要为了让错误消失就直接删 marker 或手改仓库 ID。先确认这棵 Design 为什么来自另一个仓库，保留需要的内容，再有意识地为当前仓库建立正确的 Design namespace。

如果 `docs/design/` 只是旧项目遗留、完全没有 MINE marker，则是另一种情况：`mine init` 会先自动备份，再创建新的受管 namespace。

## Plan 一直是 `BLOCKED`，无法开始

先看 Plan 和当前可执行 frontier：

```sh
mine plan show --id <id> --format json
mine graph ready --format json
```

`BLOCKED` 通常表示某个 hard predecessor 还没有到 `ACCEPTED`。先完成或补偿前置 Plan，不要手改 graph 把它硬塞成 `READY`。

如果 graph mutation 报 revision conflict，重新读取当前 graph 状态，再基于新 revision 重试原本的 transition。这个错误表示另一个合法 transition 已经先一步提交成功。

## Release closure 说 final sync 缺失或已经 stale

Phase B 要求的 sync evidence 必须对应当前最终 `dev` 状态。如果 Phase A 之后 `dev` 又发生变化，旧 evidence 就不能继续用。

重新执行 Phase A：

```text
mine-sync prepare this repository for stable release
```

然后再运行：

```text
mine-plan-review complete release closure
```

不要用 `mine design validate` 绕过 freshness 检查。Design 结构合法，不等于它已经语义上同步到最新实现。

## `mine release --format json` 返回 `can_release: false`

从返回的 `errors` 开始处理。Release preflight 是一组彼此独立的 gate，不存在一个万能修复命令。

```sh
mine release --format json
```

哪个 gate 失败，就检查哪一层：graph、Design、Git 状态、临时 release artifact 等。修完后重新跑 preflight。不要为了变绿而削弱或跳过 gate。

## 这里没有我的错误

优先看命令自己提供的帮助和机器可读诊断：

```sh
mine <command> --help
mine doctor --format json
```

如果错误可以稳定复现，而 CLI 又没有给出可操作的解释，这本身就值得报 bug。至少附上：完整命令、exit code、JSON/人类可读错误、操作系统，以及 `mine --version`。