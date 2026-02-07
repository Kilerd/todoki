from typing import Annotated

from fastapi import Depends, Header
from sqlmodel import Session

from app.config import settings
from app.core.database import get_db
from app.core.exceptions import UnauthorizedError


def verify_token(authorization: str = Header(...)) -> None:
    """Simple Bearer token verification against USER_TOKEN env var."""
    if not authorization.startswith("Bearer "):
        raise UnauthorizedError("Invalid token format")
    token = authorization[7:]
    if token != settings.user_token:
        raise UnauthorizedError()


DbSession = Annotated[Session, Depends(get_db)]
AuthDep = Annotated[None, Depends(verify_token)]
