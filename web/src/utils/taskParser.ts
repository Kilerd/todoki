


interface ParsedTaskItem {
    priority: number,
    content: string,
    group?: string,
    task_type: "Todo" | "Stateful",
    states?: string[],
}
export const parseTask = (content:string): ParsedTaskItem => {
    let item = content;
    let priority_match = item.trim().match(/(?<priority>!{1,5})/g);
    const priority = priority_match !== null ? priority_match[0] : undefined;

    let group_match = item.trim().match(/(?<group>\s\+[^+\s]+)/g);
    const group = group_match !== null ? group_match[0] : undefined;

    let states_match = item.trim().match(/\[\[([^[\]]+)]]/g);
    const states_string = states_match !== null ? states_match[0] : undefined;
    const states = states_string?.substring(2, states_string?.length - 2).split(">").map(it => it.trim())

    if (priority) {
        item = item.replace(priority, "");
    }
    if (group) {
        item = item.replace(group, "");
    }
    if (states_string) {
        item = item.replace(states_string, "");
    }

    return {
        priority: (priority ?? "").length,
        group: group?.replace(" +", ""),
        content: item.trim(),
        task_type: states === undefined ? "Todo" : "Stateful",
        states: states
    };
}