from collections.abc import AsyncGenerator
from typing import Annotated

from fastapi import Depends, Header
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.core.database import get_db
from app.core.exceptions import UnauthorizedError


async def verify_token(authorization: str = Header(...)) -> None:
    """Simple Bearer token verification against USER_TOKEN env var."""
    if not authorization.startswith("Bearer "):
        raise UnauthorizedError("Invalid token format")
    token = authorization[7:]
    if token != settings.user_token:
        raise UnauthorizedError()


async def get_db_session() -> AsyncGenerator[AsyncSession, None]:
    async for session in get_db():
        yield session


DbSession = Annotated[AsyncSession, Depends(get_db_session)]
AuthDep = Annotated[None, Depends(verify_token)]
