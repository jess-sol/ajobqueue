CREATE TYPE job_state as enum ('not-started', 'running', 'completed', 'failed');

CREATE TABLE job_queue (
    id INT primary key generated always as identity,
    uid UUID unique not null,
    type VARCHAR not null,
    data JSONB not null,
    result JSONB default null,
    state JOB_STATE default 'not-started' not null,
    created TIMESTAMPTZ not null,
    started TIMESTAMPTZ default null,
    completed TIMESTAMPTZ default null
);

CREATE INDEX job_queue_type ON job_queue (type);
