"""kanban_status

Revision ID: e21b3ff6c788
Revises: 547270e12193
Create Date: 2026-02-07 17:08:45.522477

"""
from collections.abc import Sequence

from alembic import op
import sqlalchemy as sa
from sqlalchemy.dialects import postgresql


# revision identifiers, used by Alembic.
revision: str = 'e21b3ff6c788'
down_revision: str | None = '547270e12193'
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    # Add new status column with default 'backlog'
    op.add_column('tasks', sa.Column('status', sa.String(length=50), nullable=False, server_default='backlog'))

    # Migrate data based on existing task_type and done fields:
    # - task_type=Todo, done=false  -> status='todo'
    # - task_type=Todo, done=true   -> status='done'
    # - task_type=Stateful, done=false -> status='in-progress'
    # - task_type=Stateful, done=true  -> status='done'
    op.execute("""
        UPDATE tasks SET status = CASE
            WHEN task_type = 'Todo' AND done = false THEN 'todo'
            WHEN task_type = 'Todo' AND done = true THEN 'done'
            WHEN task_type = 'Stateful' AND done = false THEN 'in-progress'
            WHEN task_type = 'Stateful' AND done = true THEN 'done'
            ELSE 'backlog'
        END
    """)

    # Remove the server default after migration
    op.alter_column('tasks', 'status', server_default=None)

    # Drop deprecated columns
    op.drop_column('tasks', 'task_type')
    op.drop_column('tasks', 'states')
    op.drop_column('tasks', 'done')
    op.drop_column('tasks', 'current_state')


def downgrade() -> None:
    # Re-add the dropped columns
    op.add_column('tasks', sa.Column('current_state', sa.String(length=255), nullable=True))
    op.add_column('tasks', sa.Column('done', sa.Boolean(), nullable=False, server_default='false'))
    op.add_column('tasks', sa.Column('states', postgresql.ARRAY(sa.String()), nullable=True))
    op.add_column('tasks', sa.Column(
        'task_type',
        sa.Enum('TODO', 'STATEFUL', name='task_type_enum', native_enum=False),
        nullable=False,
        server_default='TODO'
    ))

    # Migrate data back
    op.execute("""
        UPDATE tasks SET
            task_type = 'Todo',
            done = CASE WHEN status = 'done' THEN true ELSE false END
    """)

    # Remove server defaults
    op.alter_column('tasks', 'done', server_default=None)
    op.alter_column('tasks', 'task_type', server_default=None)

    # Drop the status column
    op.drop_column('tasks', 'status')
