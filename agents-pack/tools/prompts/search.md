# Search Tool

Search the knowledge base for relevant information.

## Input

- `query`: Natural language search query
- `scope`: Optional - "notes", "artifacts", or "all" (default: "all")
- `limit`: Optional - Maximum results (default: 10)

## Behavior

1. Embed the query using the configured embedding provider
2. Perform vector similarity search against stored content
3. Rank results by relevance score
4. Return structured results with excerpts

## Output

```json
{
  "results": [
    {
      "id": "uuid",
      "type": "note|artifact",
      "title": "...",
      "excerpt": "...",
      "relevance": 0.0-1.0,
      "trajectory_id": "uuid"
    }
  ],
  "total_matches": 42,
  "query_embedding_time_ms": 15
}
```

## Error Handling

- Empty results: Return empty array, not error
- Provider failure: Return error with provider name and status
