# Layout Report Generator

Parse keyboard layout optimization results and generate comprehensive reports.

## Usage

```bash
cd scripts/report
uv run python -m report <results.json> [OPTIONS]
```

**Options:**
- `--corpus, -c`: Corpus name for bigram frequency filtering (optional)
- `--out, -o`: Output directory (default: `<input>_layouts/`)

## Output

The tool generates:
- **CSV**: Tabular data with all metrics and worst cases
- **SVG**: Visual keyboard layout diagrams (if `.txt` file exists)
- **Markdown**: Comprehensive report with summary table, detailed metrics, and layout visualizations

## Example

```bash
# Generate reports from optimization results
uv run python -m report results.json --corpus eng_reddit_small --out my_layouts

# Output:
#   my_layouts/
#   ├── results.csv          # Metrics table
#   ├── results.md           # Full report with visualizations
#   └── svgs/                # Layout diagrams
#       ├── layout1.svg
#       └── layout2.svg
```

## Prerequisites

- Python ≥ 3.13
- [uv](https://github.com/astral-sh/uv)
- Results from the layout optimizer (JSON + optional TXT with diagrams)
