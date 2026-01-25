# Code Smells Report

## Summary

- **Errors**: 0
- **Warnings**: 96
- **Info**: 2224

## Warnings

- `scope_heap.rs:810` - Double unwrap chain
- `scope_heap.rs:821` - Double unwrap chain
- `scope_heap.rs:878` - Double unwrap chain
- `scope_heap.rs:463` - 155 line function
- `trajectory_heap.rs:975` - Double unwrap chain
- `trajectory_heap.rs:68` - 101 line function
- `trajectory_heap.rs:246` - 200 line function
- `message_heap.rs:756` - Double unwrap chain
- `message_heap.rs:768` - Double unwrap chain
- `message_heap.rs:72` - 106 line function
- `message_heap.rs:306` - 164 line function
- `conflict_heap.rs:688` - Double unwrap chain
- `conflict_heap.rs:701` - Double unwrap chain
- `conflict_heap.rs:302` - 134 line function
- `handoff_heap.rs:754` - Double unwrap chain
- `handoff_heap.rs:769` - Double unwrap chain
- `handoff_heap.rs:848` - Double unwrap chain
- `handoff_heap.rs:860` - Double unwrap chain
- `handoff_heap.rs:973` - Double unwrap chain
- `handoff_heap.rs:60` - 108 line function
- `handoff_heap.rs:317` - 144 line function
- `heap_ops.rs:193` - 172 line function
- `delegation_heap.rs:833` - Double unwrap chain
- `delegation_heap.rs:855` - Double unwrap chain
- `delegation_heap.rs:934` - Double unwrap chain
- `delegation_heap.rs:948` - Double unwrap chain
- `delegation_heap.rs:67` - 108 line function
- `delegation_heap.rs:393` - 141 line function
- `lib.rs:150` - 108 line function
- `lib.rs:258` - 127 line function
- `lib.rs:792` - 106 line function
- `lib.rs:1087` - 194 line function
- `lib.rs:1281` - 169 line function
- `lib.rs:1835` - 147 line function
- `lib.rs:2653` - 142 line function
- `lib.rs:2961` - 152 line function
- `lib.rs:4002` - 167 line function
- `lib.rs:4469` - 144 line function
- `lib.rs:5000` - 125 line function
- `lib.rs:5277` - 126 line function
- `index_ops.rs:189` - 132 line function
- `agent_heap.rs:636` - Double unwrap chain
- `agent_heap.rs:651` - Double unwrap chain
- `agent_heap.rs:713` - Double unwrap chain
- `agent_heap.rs:725` - Double unwrap chain
- `agent_heap.rs:299` - 103 line function
- `note_heap.rs:965` - Double unwrap chain
- `note_heap.rs:89` - 129 line function
- `note_heap.rs:336` - 132 line function
- `note_heap.rs:608` - 170 line function

## Duplicate Code Clusters

### arb_optional_agent_id ~ arb_optional_agent_id (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/message_heap.rs:475-480`
- `/home/eassa/projects/caliber/caliber-pg/src/delegation_heap.rs:539-544`

### arb_optional_agent_id ~ arb_optional_agent_id (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/message_heap.rs:475-480`
- `/home/eassa/projects/caliber/caliber-pg/src/agent_heap.rs:407-412`

### arb_optional_agent_id ~ arb_optional_agent_id (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/delegation_heap.rs:539-544`
- `/home/eassa/projects/caliber/caliber-pg/src/agent_heap.rs:407-412`

### arb_optional_trajectory_id ~ arb_optional_trajectory_id (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/message_heap.rs:483-488`
- `/home/eassa/projects/caliber/caliber-pg/src/conflict_heap.rs:449-454`

### arb_optional_agent_type ~ arb_optional_agent_type (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/message_heap.rs:499-508`
- `/home/eassa/projects/caliber/caliber-pg/src/handoff_heap.rs:474-484`

### arb_optional_agent_type ~ arb_optional_agent_type (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/message_heap.rs:499-508`
- `/home/eassa/projects/caliber/caliber-pg/src/delegation_heap.rs:565-574`

### arb_optional_agent_type ~ arb_optional_agent_type (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/handoff_heap.rs:474-484`
- `/home/eassa/projects/caliber/caliber-pg/src/delegation_heap.rs:565-574`

### extraction_method_to_str ~ extraction_method_to_str (similarity: 0.95)
- `/home/eassa/projects/caliber/caliber-pg/src/edge_heap.rs:360-366`
- `/home/eassa/projects/caliber/caliber-pg/src/artifact_heap.rs:586-592`

### arb_agent_type ~ arb_agent_type (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/agent_heap.rs:415-423`
- `/home/eassa/projects/caliber/caliber-agents/src/lib.rs:1372-1379`

### ttl_to_str ~ ttl_to_str (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/artifact_heap.rs:686-700`
- `/home/eassa/projects/caliber/caliber-pg/src/note_heap.rs:529-543`

### str_to_ttl ~ str_to_ttl (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/artifact_heap.rs:703-725`
- `/home/eassa/projects/caliber/caliber-pg/src/note_heap.rs:546-568`

### arb_ttl ~ arb_ttl (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-pg/src/artifact_heap.rs:914-923`
- `/home/eassa/projects/caliber/caliber-pg/src/note_heap.rs:797-807`

### find_by_kind ~ find_by_kind (similarity: 0.8)
- `/home/eassa/projects/caliber/caliber-storage/src/hybrid_dag.rs:60-66`
- `/home/eassa/projects/caliber/caliber-core/src/event.rs:582-588`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/scope_property_tests.rs:107-117`
- `/home/eassa/projects/caliber/caliber-api/tests/note_property_tests.rs:137-147`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/scope_property_tests.rs:107-117`
- `/home/eassa/projects/caliber/caliber-api/tests/trajectory_property_tests.rs:115-125`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/scope_property_tests.rs:107-117`
- `/home/eassa/projects/caliber/caliber-api/tests/artifact_property_tests.rs:203-213`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/note_property_tests.rs:137-147`
- `/home/eassa/projects/caliber/caliber-api/tests/trajectory_property_tests.rs:115-125`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/note_property_tests.rs:137-147`
- `/home/eassa/projects/caliber/caliber-api/tests/artifact_property_tests.rs:203-213`

### optional_metadata_strategy ~ optional_metadata_strategy (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/trajectory_property_tests.rs:115-125`
- `/home/eassa/projects/caliber/caliber-api/tests/artifact_property_tests.rs:203-213`

### create_test_trajectory ~ create_test_trajectory (similarity: 1.0)
- `/home/eassa/projects/caliber/caliber-api/tests/note_property_tests.rs:46-71`
- `/home/eassa/projects/caliber/caliber-api/tests/artifact_property_tests.rs:46-71`
