-- Story 4.2: Non-Resource Cost Entry
-- Adds project_expenses table for tracking non-resource costs (vendor payments, software licenses, etc.)

CREATE TABLE IF NOT EXISTS project_expenses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    category TEXT NOT NULL CHECK (category IN ('hr', 'software', 'hardware', 'overhead')),
    description TEXT NOT NULL,
    amount_idr BIGINT NOT NULL CHECK (amount_idr > 0),
    expense_date DATE NOT NULL,
    vendor TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_project_expenses_project_id ON project_expenses (project_id);
CREATE INDEX IF NOT EXISTS idx_project_expenses_project_date ON project_expenses (project_id, expense_date DESC);
CREATE INDEX IF NOT EXISTS idx_project_expenses_project_category ON project_expenses (project_id, category);
