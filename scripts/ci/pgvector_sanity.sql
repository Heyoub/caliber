CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS _caliber_vector_sanity (
    id SERIAL PRIMARY KEY,
    embedding vector(3)
);

CREATE INDEX IF NOT EXISTS _caliber_vector_sanity_ivfflat
    ON _caliber_vector_sanity
    USING ivfflat (embedding vector_l2_ops)
    WITH (lists = 1);

SELECT indexname
FROM pg_indexes
WHERE tablename = '_caliber_vector_sanity'
  AND indexname = '_caliber_vector_sanity_ivfflat';
