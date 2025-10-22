#!/usr/bin/env python3
"""
Clean French corpus files from Leipzig format.

Two-step process:
1. Use clean_uni_leipzig_corpora.py to remove line numbers and merge lines
2. Apply character filtering (preserve French accents, filter Greek/Cyrillic)
"""

import subprocess
from pathlib import Path

import typer
from typing_extensions import Annotated

from .corpus_cleaner import (
    DEFAULT_CHUNK_SIZE,
    get_allowed_characters,
    get_replacements,
    process_chunk,
    temp_file_cleanup,
)

app = typer.Typer(help="Clean French corpus files from Leipzig format")


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
    """
    Clean French corpus files from Leipzig format.

    Two-step process:
    1. Use clean_uni_leipzig_corpora.py to remove line numbers and merge lines
    2. Apply character filtering (preserve French accents, filter Greek/Cyrillic)
    """
    allowed_chars: set[str] = get_allowed_characters("french")
    replacements: dict[str, str] = get_replacements("french")
    temp_file: Path = outfile.with_suffix(outfile.suffix + ".tmp")
    chunk_size: int = DEFAULT_CHUNK_SIZE

    with temp_file_cleanup(temp_file):
        typer.echo(
            "Step 1: Processing Leipzig format (removing line numbers, merging lines)..."
        )
        script_dir = Path(__file__).parent
        leipzig_cleaner = (
            script_dir / ".." / ".." / "ngrams" / "clean_uni_leipzig_corpora.py"
        )
        result = subprocess.run(
            ["python", str(leipzig_cleaner), str(infile), str(temp_file)],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            typer.echo(f"Error: Leipzig cleaning failed: {result.stderr}")
            raise typer.Exit(1)

        typer.echo("Step 2: Applying character replacements and filtering...")
        with (
            temp_file.open("r", encoding="utf-8") as infile_handle,
            outfile.open("w", encoding="utf-8") as outfile_handle,
        ):
            while True:
                chunk = infile_handle.read(chunk_size)
                if not chunk:
                    break
                processed = process_chunk(chunk, replacements, allowed_chars)
                outfile_handle.write(processed)

    typer.echo(f"✓ Cleaned corpus written to {outfile}")


if __name__ == "__main__":
    app()
