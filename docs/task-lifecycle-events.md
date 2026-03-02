# Task Lifecycle & Event Bus 关联指南

本文档介绍 todoki 中任务状态（TaskStatus）与事件总线（Event Bus）消息之间的关联关系，帮助开发者和 PM Agent 理解完整的任务生命周期。

---

## 任务状态模型

### 简单流程

适用于快速任务，无需完整的敏捷流程：

```
Backlog → Todo → Done
```

### 敏捷流程

适用于需要完整规划、编码、评审流程的任务：

```
┌─────────────────────────────────────────────────────────────┐
│                      PLAN 阶段                               │
│  PlanPending → PlanInProgress → PlanReview → PlanDone       │
└─────────────────────────────────────────────────────────────┘
                                                │
                                                ▼
┌─────────────────────────────────────────────────────────────┐
│                     CODING 阶段                              │
│  CodingPending → CodingInProgress → CodingReview → CodingDone│
└─────────────────────────────────────────────────────────────┘
       ▲                                        │
       │ (fail)                                 ▼
┌──────┴──────────────────────────────────────────────────────┐
│                  CROSS-REVIEW 阶段                           │
│  CrossReviewPending → CrossReviewInProgress                  │
│                              │                               │
│                    ┌─────────┴─────────┐                     │
│                    ▼                   ▼                     │
│             CrossReviewPass      CrossReviewFail             │
└──────────────────────────────────────────────────────────────┘
                     │
                     ▼
                   Done
```

---

## 状态分类

| Phase | 状态 | 含义 |
|-------|------|------|
| **Simple** | `backlog` | 待办池，尚未纳入当前迭代 |
| | `todo` | 计划执行，已纳入当前迭代 |
| **Plan** | `plan-pending` | 等待开始规划 |
| | `plan-in-progress` | 规划进行中 |
| | `plan-review` | 规划文档审核中 |
| | `plan-done` | 规划完成，准备进入编码 |
| **Coding** | `coding-pending` | 等待开始编码 |
| | `coding-in-progress` | 编码进行中 |
| | `coding-review` | PR 审核中 |
| | `coding-done` | 编码完成，准备交叉评审 |
| **CrossReview** | `cross-review-pending` | 等待交叉评审 |
| | `cross-review-in-progress` | 交叉评审进行中 |
| | `cross-review-pass` | 交叉评审通过 |
| | `cross-review-fail` | 交叉评审失败，需返工 |
| **Done** | `done` | 任务完成 |

### 向后兼容状态

| 状态 | 映射到 |
|------|--------|
| `in-progress` | Coding 阶段 |
| `in-review` | Coding 阶段 |

---

## Event Bus 事件类型

### 任务生命周期事件

| Event Kind | 触发时机 | 典型状态转换 |
|------------|---------|-------------|
| `task.created` | 创建新任务 | → `backlog` 或 `todo` |
| `task.status_changed` | 任何状态变更 | 任意 → 任意 |
| `task.assigned` | 任务分配给 Agent | `*-pending` → `*-in-progress` |
| `task.completed` | 任务成功完成 | → `done` |
| `task.failed` | 任务执行失败 | 保持当前状态或回退 |
| `task.archived` | 任务归档 | `done` → archived |

### Agent 协作事件

| Event Kind | 适用阶段 | 说明 |
|------------|---------|------|
| `agent.requirement_analyzed` | Plan | 需求分析完成 |
| `agent.business_context_ready` | Plan | 业务上下文准备就绪 |
| `agent.code_review_requested` | Coding | 请求代码审核 |
| `agent.qa_test_passed` | CrossReview | QA 测试通过 |
| `agent.qa_test_failed` | CrossReview | QA 测试失败 |

### Artifact 事件

| Event Kind | 适用阶段 | 说明 |
|------------|---------|------|
| `artifact.created` | Coding | Agent 产出工件 |
| `artifact.github_pr_opened` | Coding | 创建 GitHub PR |
| `artifact.github_pr_merged` | Coding→CrossReview | PR 合并 |

---

## 状态-事件关联矩阵

### Plan 阶段

```
plan-pending
    │
    │ task.assigned (Agent 开始规划)
    ▼
plan-in-progress
    │
    │ agent.requirement_analyzed (可选)
    │ agent.business_context_ready (可选)
    │ task.status_changed → plan-review
    ▼
plan-review
    │
    │ task.status_changed → plan-done (审核通过)
    │ task.status_changed → plan-in-progress (需修改)
    ▼
plan-done
    │
    │ task.status_changed → coding-pending
    ▼
(进入 Coding 阶段)
```

### Coding 阶段

```
coding-pending
    │
    │ task.assigned (Agent 开始编码)
    ▼
coding-in-progress
    │
    │ agent.output / agent.output_batch (实时输出)
    │ artifact.created (产出文件/提交)
    │ artifact.github_pr_opened (创建 PR)
    │ task.status_changed → coding-review
    ▼
coding-review
    │
    │ agent.code_review_requested
    │ artifact.github_pr_merged (PR 合并)
    │ task.status_changed → coding-done
    ▼
coding-done
    │
    │ task.status_changed → cross-review-pending
    ▼
(进入 CrossReview 阶段)
```

### CrossReview 阶段

```
cross-review-pending
    │
    │ task.assigned (QA Agent 开始评审)
    ▼
cross-review-in-progress
    │
    ├─── agent.qa_test_passed
    │    │
    │    │ task.status_changed → cross-review-pass
    │    ▼
    │    cross-review-pass
    │    │
    │    │ task.status_changed → done
    │    │ task.completed
    │    ▼
    │    done
    │
    └─── agent.qa_test_failed
         │
         │ task.status_changed → cross-review-fail
         ▼
         cross-review-fail
         │
         │ task.status_changed → coding-pending (返工)
         ▼
         (回到 Coding 阶段)
```

---

## PM Agent 典型工作流

### 1. 接收新需求

```json
// 收到 task.created 事件
{
  "kind": "task.created",
  "agent_id": "user",
  "task_id": "uuid",
  "data": {
    "title": "实现用户登录功能",
    "description": "支持邮箱和手机号登录"
  }
}
```

**PM Agent 动作**：评估复杂度，决定使用简单流程或敏捷流程

### 2. 启动敏捷流程

```json
// 发送 task.status_changed 事件
{
  "kind": "task.status_changed",
  "agent_id": "pm-agent",
  "task_id": "uuid",
  "data": {
    "old_status": "backlog",
    "new_status": "plan-pending"
  }
}
```

### 3. 监听阶段完成

```json
// 监听 plan-done，触发进入 Coding
// 监听 coding-done，触发进入 CrossReview
// 监听 cross-review-pass，触发完成任务
```

### 4. 处理失败回退

```json
// 收到 cross-review-fail
{
  "kind": "task.status_changed",
  "data": {
    "old_status": "cross-review-in-progress",
    "new_status": "cross-review-fail"
  }
}

// PM Agent 自动触发返工
{
  "kind": "task.status_changed",
  "agent_id": "pm-agent",
  "task_id": "uuid",
  "data": {
    "old_status": "cross-review-fail",
    "new_status": "coding-pending"
  }
}
```

---

## 事件订阅建议

### PM Agent 应订阅

- `task.created` - 新任务分配
- `task.status_changed` - 状态流转监控
- `task.completed` - 完成确认
- `task.failed` - 失败处理
- `agent.qa_test_passed` / `agent.qa_test_failed` - QA 结果
- `permission.requested` - 权限请求

### Coding Agent 应订阅

- `task.assigned` - 接收任务分配
- `agent.code_review_requested` - 代码审核请求
- `permission.responsed` - 权限应答

### QA Agent 应订阅

- `task.status_changed` (filter: `new_status = cross-review-pending`)
- `artifact.github_pr_merged` - PR 合并后触发测试

---

## 最佳实践

1. **状态变更必须发送事件** - 每次调用 `update_task_status` API 时，系统会自动发送 `task.status_changed` 事件

2. **使用 task_id 关联** - 所有与任务相关的事件都应携带 `task_id` 字段，便于追踪和过滤

3. **幂等处理** - Agent 应能处理重复事件，避免重复执行

4. **失败重试** - `cross-review-fail` 应自动触发返回 `coding-pending`，而非人工干预

5. **保持向后兼容** - `in-progress` 和 `in-review` 仍然有效，映射到 Coding 阶段
