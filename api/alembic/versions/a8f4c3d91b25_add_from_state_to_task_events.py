"""add from_state to task_events

Revision ID: a8f4c3d91b25
Revises: e21b3ff6c788
Create Date: 2026-02-07 18:00:00.000000

"""
from collections.abc import Sequence

from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision: str = 'a8f4c3d91b25'
down_revision: str | None = 'e21b3ff6c788'
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.add_column('task_events', sa.Column('from_state', sa.Text(), nullable=True))


def downgrade() -> None:
    op.drop_column('task_events', 'from_state')
