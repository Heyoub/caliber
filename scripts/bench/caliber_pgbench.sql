\set random_scope random(1, 1000)
\set random_traj random(1, 1000)
\set random_agent random(1, 1000)

-- Lightweight read-only probes against core tables.
SELECT count(*) FROM caliber_trajectory;
SELECT count(*) FROM caliber_scope;
SELECT count(*) FROM caliber_turn;
SELECT count(*) FROM caliber_agent;
SELECT count(*) FROM caliber_artifact;
SELECT count(*) FROM caliber_note;

-- Touch common indexes (if data exists) without requiring specific IDs.
SELECT trajectory_id FROM caliber_trajectory ORDER BY created_at DESC LIMIT 1;
SELECT scope_id FROM caliber_scope ORDER BY created_at DESC LIMIT 1;
SELECT turn_id FROM caliber_turn ORDER BY created_at DESC LIMIT 1;
