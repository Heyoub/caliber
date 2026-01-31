# Summarize Tool

Generate concise summaries of content.

## Input

- `content`: Text to summarize (required)
- `max_tokens`: Maximum summary length (default: 500)
- `style`: "abstractive" or "extractive" (default: "abstractive")
- `focus`: Optional - Specific aspect to emphasize

## Behavior

**Abstractive**: Generate new text that captures the essence
**Extractive**: Select and combine key sentences from the original

## Guidelines

1. Preserve key facts and conclusions
2. Maintain original intent and tone
3. Remove redundancy and filler
4. Include specific numbers/dates when important

## Output

```json
{
  "summary": "...",
  "key_points": ["...", "..."],
  "original_length": 5000,
  "summary_length": 480,
  "compression_ratio": 0.096
}
```

## Error Handling

- Content too short: Return original with note
- Content empty: Return error
