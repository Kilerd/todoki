from fastapi import FastAPI

from app.routers import report, tasks

app = FastAPI(
    title="Todoki API",
    description="Task Management API",
    version="0.1.0",
)

app.include_router(tasks.router, prefix="/api")
app.include_router(report.router, prefix="/api")


@app.get("/api")
async def health_check() -> dict[str, str]:
    """Health check endpoint."""
    return {"status": "ok"}
