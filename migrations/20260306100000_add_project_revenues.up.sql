CREATE TABLE IF NOT EXISTS project_revenues (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    revenue_month DATE NOT NULL,
    amount_idr BIGINT NOT NULL CHECK (amount_idr >= 0),
    source_type TEXT NOT NULL CHECK (source_type IN ('manual', 'erp_synced', 'manual_override')),
    source_reference TEXT,
    entered_by UUID REFERENCES users(id),
    entry_date DATE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, revenue_month)
);

CREATE INDEX IF NOT EXISTS idx_project_revenues_project_month ON project_revenues (project_id, revenue_month);
CREATE INDEX IF NOT EXISTS idx_project_revenues_project_source_type ON project_revenues (project_id, source_type);
