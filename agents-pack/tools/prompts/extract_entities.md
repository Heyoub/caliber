# Extract Entities Tool

Extract structured entities from unstructured text.

## Input

- `content`: Text to analyze (required)
- `entity_types`: Optional list of types to extract (default: all)
  - "person", "organization", "location", "date", "concept", "technology"

## Behavior

1. Parse the input text
2. Identify named entities and concepts
3. Classify each entity by type
4. Extract relationships between entities
5. Return structured data

## Output

```json
{
  "entities": [
    {
      "text": "CALIBER",
      "type": "technology",
      "confidence": 0.95,
      "start_char": 0,
      "end_char": 7
    },
    {
      "text": "PostgreSQL",
      "type": "technology",
      "confidence": 0.98,
      "start_char": 45,
      "end_char": 55
    }
  ],
  "relationships": [
    {
      "subject": "CALIBER",
      "predicate": "uses",
      "object": "PostgreSQL",
      "confidence": 0.85
    }
  ],
  "entity_count": 12,
  "processing_time_ms": 230
}
```

## Entity Types

| Type | Examples |
|------|----------|
| person | "Alice", "Dr. Smith" |
| organization | "Anthropic", "ACME Corp" |
| location | "San Francisco", "AWS us-east-1" |
| date | "January 2024", "Q3 2023" |
| concept | "machine learning", "RAG" |
| technology | "PostgreSQL", "Rust", "pgrx" |
