from fastapi import APIRouter, Query

from app.deps import AuthDep, DbSession
from app.schemas.report import ReportPeriod, ReportResponse
from app.services import report_service

router = APIRouter(prefix="/report", tags=["report"])


@router.get("", response_model=ReportResponse)
def get_report(
    _: AuthDep,
    db: DbSession,
    period: ReportPeriod = Query(default=ReportPeriod.TODAY),
) -> ReportResponse:
    """Get task activity report for a given period (today/week/month)."""
    return report_service.get_report(db, period)
