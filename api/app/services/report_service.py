from sqlmodel import Session, text

from app.schemas.report import ReportPeriod, ReportResponse


def get_report(db: Session, period: ReportPeriod) -> ReportResponse:
    """
    Get task activity report for a given period.
    Aggregates task_events to count creates, dones, archives, state changes.
    """
    interval_map = {
        ReportPeriod.TODAY: "0 days",
        ReportPeriod.WEEK: "7 days",
        ReportPeriod.MONTH: "30 days",
    }
    interval = interval_map[period]

    if period == ReportPeriod.TODAY:
        date_filter = "(datetime AT TIME ZONE 'Asia/Hong_Kong')::date = (NOW() AT TIME ZONE 'Asia/Hong_Kong')::date"
    else:
        date_filter = f"datetime >= NOW() - INTERVAL '{interval}'"

    query = text(f"""
        SELECT
            COUNT(*) FILTER (WHERE event_type = 'CREATE') AS created_count,
            COUNT(*) FILTER (WHERE event_type = 'STATUS_CHANGE' AND state = 'done') AS done_count,
            COUNT(*) FILTER (WHERE event_type = 'ARCHIVED') AS archived_count,
            COUNT(*) FILTER (WHERE event_type = 'STATUS_CHANGE') AS state_changes_count,
            COUNT(*) FILTER (WHERE event_type = 'CREATE_COMMENT') AS comments_count
        FROM task_events
        WHERE {date_filter}
    """)

    result = db.exec(query)
    row = result.fetchone()

    return ReportResponse(
        period=period,
        created_count=row[0] or 0,
        done_count=row[1] or 0,
        archived_count=row[2] or 0,
        state_changes_count=row[3] or 0,
        comments_count=row[4] or 0,
    )
