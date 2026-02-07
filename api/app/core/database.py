from collections.abc import Generator

from sqlmodel import Session, create_engine

from app.config import settings

# Convert async URL to sync URL
database_url = settings.database_url.replace("postgresql+asyncpg", "postgresql+psycopg2")
engine = create_engine(database_url, echo=False)


def get_db() -> Generator[Session, None, None]:
    with Session(engine) as session:
        yield session
