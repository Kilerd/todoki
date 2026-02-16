# 关于 Agent Bus / Event Bus 的一些思考

## 核心设计

系统需要一个 channel 来记录所发生的所有事件。假设采用 JSONL 格式，包含以下必要字段：

- **kind**: 事件类型，例如 `message_sent`, `task_completed`, `agent_created`
- **time**: 事件发生的时间戳
- **agent**: 发出事件的 agent ID
- **data**: 事件的具体数据，例如消息内容、任务结果等

Channel 可以根据平台需求以 HTTP 或 WebSocket 形式暴露给 agent。个人更倾向于 HTTP/WS。假设采用 `http://event-bus`，每个 agent 都可以通过自己的 ID 和 cursor 读取 cursor 之后发生的事件，从而感知整个平台的动态。

## 基于 channel + agent-id + cursor 的能力

- **增量消费**：不同 agent 通过 cursor 找到自上次触发到现在的所有关心的 kind 事件，执行相应逻辑
- **事件回溯**：cursor 并非只能递增，可以通过 checkpoint cursor 实现事件回溯，将 cursor A 到 cursor B 的事件做聚合分析，从而改进和抽象出 skill
- **标准化交互**：Event Bus 让不同 agent 的交互标准化，解耦 agent-to-agent 的直接通信，交互流程统一为 emit → broadcast → listen

## 为什么选择 HTTP/WS 作为 broadcast 媒介

核心逻辑是让 agent 可以**多地部署、可插拔**。

假设 Event Bus 可以通过 `https://foo.bar/event-bus` 访问，接入新 agent 时无需在 Event Bus 所在机器上部署，可以在世界任意位置启动程序，通过 HTTP/WS 定时访问：

```
https://foo.bar/event-bus?agent-id=remote-agent&cursor=latest-cursor
```

即可完成对整个系统的感知。一旦返回结果中存在关心的 kind，便触发 LLM 执行任务，并通过 emit 返回结果，或发出其他 agent 所关心/监听的特定 kind 事件。

## 集成能力

通过标准的 Event Bus 抽象，可以：
- 聚合开放任务式的 claw 类 AI
- 通过简单包装完成对 claude-code、codex 等编程任务工具的集成

落地到 todoki 中，接下来会通过这种模式集成 openclaw 与 claude-code。

## 目标：基于 Kanban 的 Agent Team

todoki 目标是以 Kanban 为媒介打造 agent team。期望：人类给出一句话的业务需求，agent team 便通过 kanban + event-bus 进行 agent-to-agent 交互，完成整体需求。

设定中的分工：

| Agent | 职责 |
|-------|------|
| **PM Agent** | 分析业务需求，制定计划与排期，完成后通知 BA Agent |
| **BA Agent** | 通过 skill 加载业务背景与当前 codebase，进行业务分析、内容补全/纠正、确定变更范围与回归测试影响面 |
| **Coding Agent** | 响应 BA Agent emit 的 `agent-start` 事件，通过 ACP 协议启动 claude-code 或 codex 完成任务实现，产出 PR |
| **QA Agent** | Coding Agent 完成后触发，检测 PR 完整性并给出 review 意见 |

## 架构优势：能力解耦

上述多类型 agent 对应标准 Agile 团队的各角色。在这些岗位中，PM 的能力最复杂繁琐，claude-code 这类编程特化工具难以胜任，而 openclaw 为代表的开放任务式 AI 更适合。

这实现了 agent 层级的**能力解耦**：agent 与 agent 之间不直接通信，也不关心对方是什么类型（甚至可能是人类），只关注 Event Bus 上的事件。这使得不同任务、不同岗位采用不同类型 agent 变得可行且可靠。

## 设计灵感

感谢以下项目的启发：
- psiace 的 tape
- psiace 与 frost 的 Bub
- yetone 的 alma
- yanli 的 kapybara
- Leetao 的 huluwa
