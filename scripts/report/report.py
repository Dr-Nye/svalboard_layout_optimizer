#!/usr/bin/env python3

import csv
import json
import re
from pathlib import Path
from typing import Optional
from urllib.parse import quote

import typer
from rich.console import Console

# =============================================================================
# CONSTANTS
# =============================================================================

# Frequency and formatting
DEFAULT_FREQ_THRESHOLD = 0.01  # 1% - minimum frequency to include in reports
BALANCE_METRIC_DECIMALS = 1  # decimal places for balance metrics
DEFAULT_METRIC_DECIMALS = 2  # decimal places for most metrics

# Regex patterns (compiled once at module level)
SECTION_RE = re.compile(r"^\s*([^:;]+):\s*(.*)$")
ENTRY_RE = re.compile(
    r"(?:(?<=^)|(?<=,)|(?<=;)|(?<=:))\s*"  # entry boundary
    r"(?P<token>.+?)\s*"  # token (lazy)
    r"(?P<cost>\d+(?:\.\d+)?)%\|"  # cost%
    r"(?P<freq>\d+(?:\.\d+)?)%",  # freq%
    re.S,
)

METRICS_ORDER = [
    ("Total Cost", "total_cost", "number", 1),
    ("Hands Disbalance", "Hand Disbalance", "message_only", None),
    ("Finger Disbalance", "Finger Balance", "message_only", None),
    ("SFB", "SFB", "number", 2),
    ("Scissors", "Scissors", "number", 2),
    ("FSB", "FSB", "number", 2),
    ("HSB", "HSB", "number", 2),
    ("SFS", "SFS", "number", 2),
    ("Key Costs", "Key Costs", "number", 2),
    ("Manual Bigram Penalty", "Manual Bigram Penalty", "number", 2),
    ("SFB Worst", "SFB", "worst_only", None),
    ("Scissors Worst", "Scissors", "worst_only", None),
    ("FSB Worst", "FSB", "worst_only", None),
    ("HSB Worst", "HSB", "worst_only", None),
    ("SFS Worst", "SFS", "worst_only", None),
    ("Movement Pattern Worst", "Movement Pattern", "worst_only", None),
    ("Manual Bigram Penalty Worst", "Manual Bigram Penalty", "worst_only", None),
    ("Secondary Bigrams Worst", "Secondary Bigrams", "worst_only", None),
    ("Trigrams Worst", "No Handswitch in Trigram", "worst_only", None),
    ("Movement Pattern", "Movement Pattern", "number", 2),
    ("Bigram Statistics", "Bigram Statistics", "message_only", None),
    ("Trigram Statistics", "Trigram Statistics", "message_only", None),
]

COLUMN_HEADERS = ["Layout", "Homerow"] + [display for display, *_ in METRICS_ORDER]

METRICS_DESCRIPTION = """## Metrics Description

**finger_balance**: Left pinky -> left index and then right index -> right pinky

**hand_disbalance**: Left and right hand balance

**bigram_stats**: Informational statistics showing percentages of various bigram categories:
  - **SFB**: Same Finger Bigrams (excluding good Center→South movements)
  - **Full Vertical**: North-South scissoring opposition
  - **Squeeze**: Fingers moving inward (In↔Out opposition, more uncomfortable)
  - **Splay**: Fingers moving outward (Out↔In opposition, less uncomfortable)
  - **Half**: Diagonal movements (lateral + vertical)
  - **Lateral**: Lateral displacement with center

**direction_balance**: Tracks keypress patterns in different directions (informational only). Center and south keys are ideal

**key_costs**: Penalizes using keys that are harder to reach based on position (based on direction and finger)

**position_penalties**: Applies penalties when specific characters appear at specific matrix positions. Used to enforce character placement constraints, such as restricting high-frequency double letters to comfortable positions or keeping punctuation marks off homerow keys

**sfb**: Same Finger Bigram metric that evaluates the comfort of same finger bigrams. Center to south bigrams are good here.

**sfs**: Same Finger Skipgram (SFS) metric that evaluates the comfort of skipgrams typed with the same finger. Skipgrams are generally uncomfortable because the middle keystroke interrupts the finger movement pattern.

**scissors**: Cost-based scissor metric that penalizes adjacent finger movements where there's an effort imbalance (e.g., weak finger doing hard work while strong finger gets easy work). Penalties scale proportionally to the key cost difference and distinguish between movement types: Full Scissor Vertical (North↔South), Full Scissor Squeeze/Splay (In↔Out lateral, squeeze being worse), Half Scissor (diagonal lateral+vertical), and Lateral Stretch (lateral+center)

**fsb**: Full Scissor Bigram metric focused on the most uncomfortable scissor movements: Full Scissor Vertical (North↔South opposition), Full Scissor Squeeze (fingers moving inward), and Full Scissor Splay (fingers moving outward). Penalties scale proportionally to the key cost difference

**hsb**: Half Scissor Bigram metric focused on less severe scissor movements: Half Scissor (diagonal lateral+vertical movements) and Lateral (lateral+center movements). Penalties scale proportionally to the key cost difference

**symmetric_handswitches**: Rewards using symmetrical key positions when switching between hands, but only for center, south, and index/middle north keys

**movement_pattern**: Assigns costs to finger transitions within the same hand. If the movement is center key to center key or south key to south key, there is no penalty

**manual_bigram_penalty**: Applies specific penalties to manually defined key combinations that are hard to describe otherwise, such as same-key repeats on pinky fingers

**secondary_bigrams**: Evaluates the comfort of the first and last keys in three-key sequences

**no_handswitch_in_trigram**: Penalizes typing three consecutive keys on the same hand

**trigram_statistics**: Provides statistics on rolls and redirects:
  - **Bigram roll in/out**: Rolls are 3-key sequences (2,1 or 1,2) with hand alternation where the 2 same-hand keys use different fingers. Inward rolls move towards the index finger, outward rolls move towards the pinky
  - **Center->South**: Same-finger vertical movement from center row to south row in a roll pattern
  - **Redirect**: One-handed trigrams where direction changes (e.g., QWERTY "SAD": outward then inward)
  - **Weak redirect**: Redirects that don't involve the index finger, considered harder to type

"""

# =============================================================================
# CORPUS HANDLING
# =============================================================================


def get_corpus_paths(corpus_name: str) -> tuple[Path, Path, Path]:
    """Get corpus directory and ngrams file paths."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    corpus_dir = project_root / "ngrams" / corpus_name
    ngrams_file = corpus_dir / "2-grams.txt"
    return project_root, corpus_dir, ngrams_file


def load_bigram_frequencies(corpus_name: str) -> dict[str, float]:
    """Load bigram frequencies from corpus 2-grams.txt file."""
    _, _, ngrams_file = get_corpus_paths(corpus_name)

    frequencies = {}
    with open(ngrams_file, encoding="utf-8") as f:
        for line in f:
            parts = line.strip().split(" ")
            if len(parts) >= 2:
                freq = float(parts[0])
                bigram = parts[1]
                if len(bigram) == 2:
                    frequencies[bigram] = freq
    return frequencies


def validate_corpus(corpus_name: str) -> str:
    """Validate that corpus exists and return the corpus name."""
    if not corpus_name:
        return corpus_name

    project_root, corpus_dir, _ = get_corpus_paths(corpus_name)

    if not corpus_dir.exists():
        ngrams_dir = project_root / "ngrams"
        available_corpora = (
            sorted([d.name for d in ngrams_dir.iterdir() if d.is_dir()])
            if ngrams_dir.exists()
            else []
        )
        available = (
            f" Available: {', '.join(available_corpora)}" if available_corpora else ""
        )
        raise typer.BadParameter(f"Corpus '{corpus_name}' not found.{available}")

    return corpus_name


# =============================================================================
# MESSAGE PROCESSING
# =============================================================================


def drop_low_freq_entries(
    message: str, threshold: float = DEFAULT_FREQ_THRESHOLD
) -> str:
    """
    Remove entries whose frequency (right side of '|') is < threshold (%).
    Preserves tokens that include punctuation such as commas (e.g., 'l,', 'o,', 'a.').
    Drops empty sections.
    """
    out_sections = []
    for raw_sec in (s.strip() for s in message.split(";")):
        if not raw_sec:
            continue
        m = SECTION_RE.match(raw_sec)
        if m:
            label, body = m.group(1).strip(), m.group(2)
        else:
            label, body = None, raw_sec

        kept = []
        for em in ENTRY_RE.finditer(body):
            token = em.group("token").strip()  # keep punctuation like ',' or '.'
            cost = em.group("cost")
            freq = float(em.group("freq"))
            if freq >= threshold:
                kept.append(f"{token} {cost}%|{em.group('freq')}%")

        if kept:
            out_sections.append(
                f"{label}: {', '.join(kept)}" if label else ", ".join(kept)
            )

    return "; ".join(out_sections)


def clean_worst_message(message: str, metric_name: str = "") -> str:
    """Clean message by removing unnecessary prefixes."""
    prefixes = ["Finger loads % (no thumb): ", "Hand loads % (no thumb): ", "Worst: "]
    for prefix in prefixes:
        message = message.replace(prefix, "")

    # Format percentages and other numbers
    if metric_name in ["Hand Disbalance", "Finger Balance"]:
        decimals = BALANCE_METRIC_DECIMALS
        message = re.sub(
            r"(\d+\.\d+)(?!%\))", lambda m: f"{float(m.group(1)):.{decimals}f}", message
        )
    else:
        decimals = DEFAULT_METRIC_DECIMALS
        message = re.sub(
            r"(\d+\.\d+)%,", lambda m: f"{float(m.group(1)):.{decimals}f}%,", message
        )

    return message.strip()


# =============================================================================
# LAYOUT PARSING AND PROCESSING
# =============================================================================


def process_layout_metrics(
    result: dict, bigram_frequencies: dict[str, float]
) -> dict[str, dict]:
    """Process all metrics for a single layout result."""
    metrics_data = {}

    for individual_result in result["details"]["individual_results"]:
        for metric_cost in individual_result["metric_costs"]:
            core = metric_cost["core"]
            message = core["message"]

            if (
                core["name"]
                in [
                    "SFB",
                    "Manual Bigram Penalty",
                    "Scissors",
                    "FSB",
                    "HSB",
                ]
                and bigram_frequencies
            ):
                message = drop_low_freq_entries(message)

            metrics_data[core["name"]] = {
                "cost": metric_cost["weighted_cost"],
                "message": message,
            }

    return metrics_data


def extract_homerow(layout: str) -> str:
    """Extract center keys (homerow) from layout string.

    Layout is 8 clusters of 5 chars + 1 thumb.
    Each cluster: N O C I S (North, Out, Center, In, South)
    Center is at position 2 in each cluster (0-indexed).
    Returns centers from left ring to right ring (6 characters, excluding pinkies).
    """
    if len(layout) < 40:
        return ""

    centers = []
    # Clusters 1-6: left ring, left middle, left index, right index, right middle, right ring
    # (skipping cluster 0 = left pinky and cluster 7 = right pinky)
    for cluster_idx in range(0, 8):
        center_pos = cluster_idx * 5 + 2
        centers.append(layout[center_pos])

    return "".join(centers)


def build_layout_row(
    layout: str, total_cost: float, metrics_data: dict[str, dict]
) -> dict:
    """Build a dict for one row following COLUMN_HEADERS order.
    Insertion order matches COLUMN_HEADERS.
    """
    row = {}
    row[COLUMN_HEADERS[0]] = layout  # "Layout"
    row[COLUMN_HEADERS[1]] = extract_homerow(layout)  # "Homerow"

    for display_header, metric_name, format_type, decimals in METRICS_ORDER:
        if format_type == "number" and display_header == "Total Cost":
            row[display_header] = round(total_cost, decimals)
        elif format_type == "number" and metric_name in metrics_data:
            cost = metrics_data[metric_name]["cost"]
            row[display_header] = round(cost, decimals)
        elif (
            format_type in ("message_only", "worst_only")
            and metric_name in metrics_data
        ):
            message = clean_worst_message(
                metrics_data[metric_name]["message"], metric_name
            )
            row[display_header] = message
        else:
            row[display_header] = ""
    return row


def parse_layouts(json_file: Path, corpus_name: Optional[str] = None) -> list[dict]:
    """Load results and build a list of dict rows, sorted by total cost."""

    with open(json_file, encoding="utf-8") as f:
        data = json.load(f)

    bigram_frequencies = load_bigram_frequencies(corpus_name) if corpus_name else {}
    sorted_data = sorted(data, key=lambda x: x["total_cost"])

    records: list[dict] = []
    for result in sorted_data:
        layout = result["details"]["layout"]
        total_cost = result["total_cost"]
        metrics_data = process_layout_metrics(result, bigram_frequencies)
        records.append(build_layout_row(layout, total_cost, metrics_data))
    return records


# =============================================================================
# LAYOUT DIAGRAM PARSING AND SVG GENERATION
# =============================================================================


def parse_layout_diagram(text: str) -> list[str]:
    """Parse layout diagram from text and return as list of lines."""
    lines = text.split("\n")
    layout_lines = []

    start_idx = next(
        (i + 1 for i, line in enumerate(lines) if "Layout (layer 1):" in line), None
    )
    if start_idx is None:
        return []

    for line in lines[start_idx:]:
        if "Layout string" in line:
            break
        if line.strip():
            layout_lines.append(line)

    return layout_lines


def export_svg(layout_lines: list[str], output_path: Path) -> None:
    """Create SVG representation of the keyboard layout using Rich."""
    console = Console(record=True, width=64)

    for line in layout_lines:
        styled_line = ""
        for char in line:
            if char == "□":
                styled_line += f"[gray]{char}[/gray]"
            elif char.isalpha():
                styled_line += f"[yellow]{char}[/yellow]"
            else:
                styled_line += char
        console.print(styled_line)

    console.save_svg(output_path, title="", font_aspect_ratio=1)


def parse_diagrams(txt_file: Path, output_dir: Path) -> list[tuple[str, str]]:
    """Parse results.txt file and generate SVG files for each layout."""
    if not txt_file.exists():
        raise FileNotFoundError(f"Results file not found: {txt_file}")

    output_path = Path(output_dir)
    output_path.mkdir(exist_ok=True)

    with open(txt_file, encoding="utf-8") as f:
        content = f.read()

    layout_sections = []
    current_section = []

    for line in content.split("\n"):
        if line.startswith("Layout (layer 1):") and current_section:
            layout_sections.append("\n".join(current_section))
            current_section = [line]
        else:
            current_section.append(line)

    if current_section:
        layout_sections.append("\n".join(current_section))

    generated_layouts = []

    for section in layout_sections:
        layout_string_match = re.search(r"Layout string \(layer 1\):\n(.+)", section)
        if not layout_string_match:
            continue

        layout_string = layout_string_match.group(1).strip()
        layout_lines = parse_layout_diagram(section)

        if not layout_lines:
            continue

        svg_path = output_path / f"{layout_string}.svg"
        export_svg(layout_lines, svg_path)
        typer.echo(f"Generated: {svg_path}")
        generated_layouts.append((layout_string, str(svg_path)))

    return generated_layouts


# =============================================================================
# MARKDOWN HELPERS
# =============================================================================


def generate_anchor_id(text: str) -> str:
    """Generate a markdown anchor ID from text (GitHub-flavored markdown style)."""
    # Convert to lowercase
    anchor = text.lower()
    # Replace spaces with hyphens
    anchor = anchor.replace(" ", "-")
    # Remove special characters, keeping only alphanumeric, hyphens, and underscores
    anchor = re.sub(r"[^a-z0-9\-_]", "", anchor)
    # Remove consecutive hyphens
    anchor = re.sub(r"-+", "-", anchor)
    # Strip leading/trailing hyphens
    anchor = anchor.strip("-")
    return anchor


# =============================================================================
# COLUMN FILTERING
# =============================================================================


def filter_empty_columns(records: list[dict]) -> list[str]:
    """Return list of column headers that have non-empty values based on first record."""
    if not records:
        return COLUMN_HEADERS

    first_record = records[0]
    non_empty_headers = []
    for header in COLUMN_HEADERS:
        if str(first_record.get(header, "")).strip() != "":
            non_empty_headers.append(header)

    return non_empty_headers


# =============================================================================
# EXPORT FUNCTIONS
# =============================================================================


def export_csv(records: list[dict], output_file: Path) -> None:
    """Export parsed layout records to CSV file."""
    if not records:
        return

    filtered_headers = filter_empty_columns(records)
    with open(output_file, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=filtered_headers, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(records)


def export_markdown(
    records: list[dict],
    generated_layouts: list[tuple[str, str]],
    output_file: Path,
) -> None:
    """Export parsed layout records to markdown with summary table and detailed sections."""
    filtered_headers = filter_empty_columns(records)

    with open(output_file, "w", encoding="utf-8") as f:
        f.write("# Keyboard Layout Results\n\n")

        toc_items = (
            ["- [Summary](#summary)", "- [Layout Details](#layout-details)"]
            + [
                f"  - [{rec['Layout']}](#{generate_anchor_id(rec['Layout'])})"
                for rec in records
            ]
            + ["- [Metrics Description](#metrics-description)"]
        )
        f.write("## Table of Contents\n\n")
        f.write("\n".join(toc_items) + "\n\n")

        f.write("## Summary\n\n")

        # Filter summary headers to only include those with data
        all_summary_headers = [
            "SVG",
            "Homerow",
            "Total Cost",
            "Hands Disbalance",
            "Finger Disbalance",
            "Bigram Stats",
            "Non-Rolls",
            "SFB",
            "SFS",
            "Scissors",
            "Manual Bigram Penalty",
            "Layout",
        ]

        # Map display names to actual column names for filtering
        summary_mapping = {
            "SVG": "SVG",  # Special case, always included if SVGs exist
            "Homerow": "Homerow",  # Always included
            "Total Cost": "Total Cost",
            "Hands Disbalance": "Hands Disbalance",
            "Finger Disbalance": "Finger Disbalance",
            "Bigram Stats": "Bigram Stats",
            "Non-Rolls": "Non-Rolls",
            "SFB": "SFB",
            "SFS": "SFS",
            "Scissors": "Scissors",
            "Manual Bigram Penalty": "Manual Bigram Penalty",
            "Layout": "Layout",  # Always included
        }

        summary_headers = []
        metrics = []

        for header in all_summary_headers:
            if header in ["SVG", "Layout", "Homerow"]:
                # Always include SVG, Homerow, and Layout columns
                summary_headers.append(header)
                if header not in ["SVG", "Layout"]:
                    metrics.append(summary_mapping[header])
            elif summary_mapping[header] in filtered_headers:
                summary_headers.append(header)
                metrics.append(summary_mapping[header])

        f.write("| " + " | ".join(summary_headers) + " |\n")
        f.write("|" + "|".join(["--------"] * len(summary_headers)) + "|\n")

        layout_to_svg = dict(generated_layouts) if generated_layouts else {}
        for rec in records:
            layout = rec["Layout"]
            svg_cell = (
                f'<img src="svgs/{quote(Path(layout_to_svg[layout]).name)}" width="600">'
                if layout in layout_to_svg
                else ""
            )
            layout_link = f"[{layout}](#{generate_anchor_id(layout)})"
            row_cells = (
                [svg_cell]
                + [str(rec.get(metric, "")) for metric in metrics]
                + [layout_link]
            )
            f.write("| " + " | ".join(row_cells) + " |\n")

        f.write("\n## Layout Details\n\n")
        for rec in records:
            layout = rec["Layout"]
            f.write(f"### {layout}\n\n")

            # Add SVG image if available
            if layout in layout_to_svg:
                svg_filename = Path(layout_to_svg[layout]).name
                f.write(f'<img src="svgs/{quote(svg_filename)}" width="800">\n\n')

            f.write(f"**Total Cost:** {rec.get('Total Cost', '')}\n\n")
            f.write("#### All Metrics\n\n")

            metrics_data = [
                (header, str(rec.get(header, "")))
                for header in filtered_headers[1:]
                if "Worst" not in header
                and header != "Total Cost"
                and rec.get(header, "")
            ]
            if metrics_data:
                metric_names, values = zip(*metrics_data)
                f.write("| " + " | ".join(metric_names) + " |\n")
                f.write("|" + "|".join(["--------"] * len(metric_names)) + "|\n")
                f.write("| " + " | ".join(values) + " |\n")

            worst_cases = [
                (header.replace(" Worst", ""), rec.get(header, ""))
                for header in filtered_headers[1:]
                if "Worst" in header and rec.get(header, "")
            ]
            if worst_cases:
                f.write("\n#### Worst Cases\n\n")
                for metric_name, value in worst_cases:
                    f.write(f"- **{metric_name}:** {value}\n")

            f.write("\n---\n\n")

        f.write(f"{METRICS_DESCRIPTION}")


# =============================================================================
# CLI APPLICATION
# =============================================================================

app = typer.Typer(
    help="Parse keyboard layout optimization results and generate CSV, SVG, and markdown outputs.",
    add_completion=False,
)


@app.command()
def main(
    json_file: Path = typer.Argument(
        ...,
        help="JSON file with optimization results",
        exists=True,
        dir_okay=False,
        readable=True,
    ),
    out: Optional[str] = typer.Option(
        None,
        "--out",
        "-o",
        help="Output directory (default: derived from input file)",
    ),
    corpus: Optional[str] = typer.Option(
        None,
        "--corpus",
        "-c",
        help="Name of the corpus for bigram frequencies",
        callback=lambda x: validate_corpus(x) if x else x,
    ),
) -> None:
    """Parse keyboard layout results and generate outputs. Automatically generates SVG files and markdown table if corresponding .txt file exists."""
    txt_file = json_file.with_suffix(".txt")

    if out:
        output_dir = Path(out)
        output_base = output_dir.name
    else:
        output_base = json_file.stem
        output_dir = Path(f"{output_base}_layouts")

    output_dir.mkdir(parents=True, exist_ok=True)

    records = parse_layouts(json_file, corpus)

    csv_file = output_dir / f"{output_base}.csv"
    typer.echo(f"Generating CSV: {csv_file}")
    export_csv(records, csv_file)

    if txt_file.exists():
        typer.echo(f"Found {txt_file}, generating SVG files and markdown table...")

        svg_dir = output_dir / "svgs"
        typer.echo(f"Generating SVG files in {svg_dir}...")
        generated_layouts = parse_diagrams(txt_file, svg_dir)

        markdown_file = output_dir / f"{output_base}.md"
        typer.echo(f"Generating markdown table: {markdown_file}")
        export_markdown(records, generated_layouts, markdown_file)


if __name__ == "__main__":
    app()
