# Create Event Modal Component

前端可视化工具，用于在浏览器中快速创建和发布事件到 Event Bus。

---

## 功能特性

- ✅ **可视化表单** - 无需记忆 API 或 curl 命令
- ✅ **预定义事件类型** - 快速选择常用事件类型
- ✅ **自定义事件** - 支持任意事件类型
- ✅ **JSON 编辑器** - 编辑事件数据 payload
- ✅ **可选字段** - task_id 和 session_id 可选
- ✅ **实时反馈** - 成功/错误提示
- ✅ **自动认证** - 使用 localStorage token

---

## 使用位置

访问 **Event Stream 页面**：
```
http://localhost:5201/events
```

点击右上角的 **"Create Event"** 按钮。

---

## 表单字段

### 1. Event Kind (必填)

**预定义选项：**

**Task Events:**
- `task.created` - 任务创建
- `task.updated` - 任务更新
- `task.completed` - 任务完成
- `task.failed` - 任务失败

**Agent Events:**
- `agent.started` - Agent 启动
- `agent.stopped` - Agent 停止
- `agent.requirement_analyzed` - 需求分析完成

**Artifact Events:**
- `artifact.created` - Artifact 创建
- `artifact.github_pr_opened` - GitHub PR 打开

**Permission Events:**
- `permission.requested` - 权限请求
- `permission.approved` - 权限批准

**System Events:**
- `system.relay_connected` - Relay 连接
- `system.relay_disconnected` - Relay 断开

**自定义事件：**
直接在 "Or Custom Event Kind" 输入框输入任意事件类型，例如：
- `custom.user.login`
- `integration.webhook.received`
- `notification.sent`

### 2. Task ID (可选)

关联特定任务的 UUID，例如：
```
550e8400-e29b-41d4-a716-446655440000
```

**用途：**
- 在任务详情页筛选该任务的所有事件
- 建立事件与任务的关联关系

### 3. Session ID (可选)

关联特定会话的 UUID，例如：
```
550e8400-e29b-41d4-a716-446655440001
```

**用途：**
- 追踪同一会话的多个事件
- 用于 agent 执行过程的会话管理

### 4. Event Data (必填)

JSON 格式的事件数据，示例：

**简单数据：**
```json
{
  "content": "Implement user authentication"
}
```

**复杂数据：**
```json
{
  "content": "Implement user authentication",
  "priority": "high",
  "estimated_effort": "3 days",
  "tags": ["backend", "security"],
  "metadata": {
    "created_by": "user-123",
    "source": "web-ui"
  }
}
```

**Agent 分析结果：**
```json
{
  "plan": "1. Design API, 2. Implement auth, 3. Write tests",
  "estimated_effort": "high",
  "dependencies": ["database", "jwt-library"]
}
```

---

## 使用示例

### 示例 1: 创建任务事件

1. 点击 "Create Event" 按钮
2. 选择 Event Kind: `task.created`
3. 填写 Task ID: `550e8400-e29b-41d4-a716-446655440000`
4. 填写 Event Data:
   ```json
   {
     "content": "Implement dark mode",
     "priority": "medium"
   }
   ```
5. 点击 "Create Event"
6. 查看 Event Stream 页面，新事件会立即出现

### 示例 2: Agent 需求分析事件

1. Event Kind: `agent.requirement_analyzed`
2. Task ID: `550e8400-e29b-41d4-a716-446655440000`
3. Event Data:
   ```json
   {
     "analysis": "User wants dark mode toggle in settings page",
     "technical_requirements": [
       "CSS variables for theme colors",
       "localStorage for persistence",
       "Theme context provider"
     ],
     "estimated_time": "4 hours"
   }
   ```

### 示例 3: 自定义通知事件

1. 在 "Or Custom Event Kind" 输入: `notification.email.sent`
2. Event Data:
   ```json
   {
     "recipient": "user@example.com",
     "subject": "Task completed",
     "sent_at": "2026-02-28T10:30:00Z"
   }
   ```

---

## 验证事件创建

创建事件后，可以通过以下方式验证：

### 1. 前端 Event Stream 页面
事件会立即出现在实时事件流中（如果过滤器匹配）。

### 2. WebSocket 实时推送
打开浏览器控制台，查看 WebSocket 消息：
```
[EventStream] Received event: task.created (cursor: 123)
```

### 3. HTTP API 查询
```bash
curl "http://localhost:8201/api/event-bus?cursor=0&limit=5" \
  -H "Authorization: Bearer change-me-in-production"
```

---

## 组件集成

在其他页面集成 CreateEventModal：

```typescript
import { CreateEventModal } from '@/components/CreateEventModal';

function MyPage() {
  return (
    <div>
      <h1>My Page</h1>

      {/* Simple integration */}
      <CreateEventModal />

      {/* With callback */}
      <CreateEventModal
        onEventCreated={(cursor) => {
          console.log(`Event created with cursor: ${cursor}`);
          // Refresh data, show notification, etc.
        }}
      />
    </div>
  );
}
```

---

## 错误处理

### 认证错误
**错误信息：** "Authentication token not found"

**解决方案：**
```javascript
// 在浏览器控制台设置 token
localStorage.setItem('todoki_auth_token', 'change-me-in-production');
```

### JSON 解析错误
**错误信息：** "Invalid JSON in data field"

**解决方案：**
- 检查 JSON 语法（逗号、引号、括号）
- 使用 JSON 验证工具验证格式
- 常见错误：
  - 最后一个字段后多余的逗号
  - 单引号而不是双引号
  - 未转义的特殊字符

### 网络错误
**错误信息：** "Failed to emit event: ..."

**解决方案：**
1. 检查后端是否运行 (`http://localhost:8201/health`)
2. 检查浏览器控制台 Network 标签
3. 验证 token 是否正确
4. 检查 CORS 配置

---

## 技术实现

### API 客户端
文件: `web/src/api/eventBus.ts`

```typescript
export async function emitEvent(
  request: EmitEventRequest
): Promise<EmitEventResponse>
```

### Modal 组件
文件: `web/src/components/CreateEventModal.tsx`

**特性：**
- shadcn/ui Dialog 组件
- Form validation
- Loading states
- Success/Error alerts
- Auto-reset after submission

---

## 最佳实践

### 1. 使用有意义的事件类型
```
✅ task.created
✅ agent.requirement_analyzed
❌ event1
❌ test
```

### 2. 结构化的事件数据
```json
✅ {
  "content": "Task description",
  "priority": "high",
  "tags": ["backend"]
}

❌ {
  "data": "Task description high backend"
}
```

### 3. 保持数据简洁
只包含必要信息，避免冗余数据。

### 4. 使用 Task ID 建立关联
当事件与任务相关时，始终填写 Task ID。

---

## 快捷测试工作流

1. 打开 Event Stream 页面
2. 在左侧过滤器选择事件类型（如 "Task Events"）
3. 点击 "Create Event" 创建测试事件
4. 观察事件实时出现在右侧时间线
5. 展开事件查看详细数据

---

## 相关文档

- **Event Bus 系统概述：** `docs/EVENTTIMELINE_FINAL_SUMMARY.md`
- **EventTimeline 使用指南：** `docs/EVENTTIMELINE_USAGE_GUIDE.md`
- **Phase 2 快速开始：** `docs/PHASE2_QUICKSTART.md`
- **WebSocket API：** `crates/todoki-server/src/api/event_bus_ws.rs`
- **HTTP API：** `crates/todoki-server/src/api/event_bus.rs`

---

**Last Updated:** 2026-02-28
