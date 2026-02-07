interface ParsedTaskItem {
    priority: number,
    content: string,
    group?: string,
}

export const parseTask = (content: string): ParsedTaskItem => {
    let item = content;
    let priority_match = item.trim().match(/(?<priority>!{1,5})/g);
    const priority = priority_match !== null ? priority_match[0] : undefined;

    let group_match = item.trim().match(/(?<group>\s\+[^+\s]+)/g);
    const group = group_match !== null ? group_match[0] : undefined;

    if (priority) {
        item = item.replace(priority, "");
    }
    if (group) {
        item = item.replace(group, "");
    }

    return {
        priority: (priority ?? "").length,
        group: group?.replace(" +", ""),
        content: item.trim(),
    };
}
