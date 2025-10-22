#!/usr/bin/env python3
"""
Clean Reddit corpus text for English keyboard layout optimization.

Apply character filtering and line merging:
1. Apply character replacements (normalize punctuation, convert accents)
2. Filter out characters not typable on standard US keyboard
3. Merge lines (replace 4 out of 5 newlines with spaces, keeping 1 in 5)
"""

import re
from pathlib import Path

import typer
from typing_extensions import Annotated

from .corpus_cleaner import (
    DEFAULT_CHUNK_SIZE,
    DEFAULT_MERGE_LINES_RATIO,
    REDDIT_ALLOWED_CHARS,
    BASE_REPLACEMENTS,
    process_chunk,
    merge_lines,
    temp_file_cleanup,
)

app = typer.Typer(help="Clean Reddit corpus for English keyboard layout optimization")


@app.command()
def main(
    infile: Annotated[
        Path,
        typer.Argument(
            help="Corpus file to process",
            exists=True,
            file_okay=True,
            dir_okay=False,
            readable=True,
        ),
    ],
    outfile: Annotated[Path, typer.Argument(help="Result filename")],
) -> None:
    """Clean Reddit corpus for English keyboard layout optimization.

    Applies character filtering and line merging to prepare text for
    generating n-gram frequency tables for keyboard layout analysis.
    """
    allowed_chars: set[str] = REDDIT_ALLOWED_CHARS
    replacements: dict[str, str] = BASE_REPLACEMENTS
    chunk_size: int = DEFAULT_CHUNK_SIZE
    merge_lines_ratio: float = DEFAULT_MERGE_LINES_RATIO

    intermediate_file = outfile.with_suffix(outfile.suffix + ".intermediate")

    with temp_file_cleanup(intermediate_file):
        typer.echo("Step 1: Applying character replacements and filtering...")
        with (
            infile.open("r", encoding="utf-8") as infile_handle,
            intermediate_file.open("w", encoding="utf-8") as outfile_handle,
        ):
            while True:
                chunk = infile_handle.read(chunk_size)
                if not chunk:
                    break
                chunk = chunk.lower()
                processed = process_chunk(chunk, replacements, allowed_chars)
                outfile_handle.write(processed)

        typer.echo(
            f"Step 2: Merging lines ({int(merge_lines_ratio * 100)}% newlines kept)..."
        )
        text = intermediate_file.read_text(encoding="utf-8")
        text = merge_lines(text, merge_lines_ratio)

        typer.echo("Step 3: Removing remaining newlines...")
        text = text.replace("\n", "")

        typer.echo("Step 4: Removing double spaces...")
        text = re.sub(r" +", " ", text)

        outfile.write_text(text, encoding="utf-8")

    typer.echo(f"✓ Cleaned corpus written to {outfile}")


if __name__ == "__main__":
    app()
