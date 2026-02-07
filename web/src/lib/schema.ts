
export interface ItemLineItem {
    id: string
    priority: number
    content: string
    group: string
    task_type: "Todo" | "Stateful"
    create_at: string
    events: TaskEvent[]
    done: boolean
    archived: boolean
    comments: string[]
    current_state: any
    states: string[]
    habit_id?: string
    habit_name?: string
}

export interface TaskEvent {
    id: string
    task_id: string
    event_type: string
    datetime: string
    state: any
}