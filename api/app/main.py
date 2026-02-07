from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.deps import AuthDep
from app.routers import report, tasks

app = FastAPI(
    title="Todoki API",
    description="Task Management API",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(tasks.router, prefix="/api")
app.include_router(report.router, prefix="/api")


@app.get("/api")
async def health_check(_: AuthDep) -> dict[str, str]:
    """Health check endpoint."""
    return {"status": "ok"}
