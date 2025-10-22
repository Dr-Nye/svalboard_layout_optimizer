#!/usr/bin/env python3
"""
Shared corpus cleaning utilities for keyboard layout optimization.

This module provides common functionality for cleaning text corpora by:
- Normalizing punctuation (smart quotes, dashes) to ASCII
- Deleting special symbols without inflating character counts
- Filtering characters based on language-specific allowed sets
- Processing large files in chunks for memory efficiency

Philosophy:
- Use consistent normalization across all corpora for comparable statistics
- Avoid artificial character count inflation (em-dash → "-", not "--")
- Delete symbols rare in natural text rather than converting to multi-char
"""

import re
import itertools
import subprocess
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator, Literal

import typer
from typing_extensions import Annotated

Language = Literal["english", "french"]

# Processing constants
DEFAULT_CHUNK_SIZE = 50 * 1024 * 1024  # 50MB
DEFAULT_MERGE_LINES_RATIO = 0.2  # Keep 1 in 5 newlines

# Base typable characters (standard US keyboard)
TYPABLE_CHARS = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890"
TYPABLE_CHARS += r""",.!?;:_'"^~#%&/\()[]{}<>=+-*`@$€|"""
TYPABLE_CHARS += " \t\n"

# French-specific characters
FRENCH_CHARS = "àâäæçéèêëïîôùûüÿœÀÂÄÆÇÉÈÊËÏÎÔÙÛÜŸŒ"

# Common character replacements (shared across all languages)
# Philosophy: Normalize to ASCII equivalents without inflating character counts
BASE_REPLACEMENTS = {
    # Normalize quotes
    "'": "'",
    "´": "'",
    "`": "'",
    """: '"',
    """: '"',
    "«": '"',
    "»": '"',
    # Normalize dashes to single hyphen
    "‒": "-",
    "–": "-",
    "—": "-",
    "―": "-",
    "−": "-",
    "─": "-",
    # Special punctuation
    "…": ".",
    # Delete symbols (rare in natural text)
    "•": "",
    "·": "",
    "™": "",
    "®": "",
    "©": "",
    # Delete superscripts/subscripts
    "¹": "",
    "²": "",
    "³": "",
    "⁴": "",
    "⁰": "",
    # Delete fractions
    "½": "",
    "¼": "",
    "¾": "",
    "⅓": "",
    "⅔": "",
    # Delete math symbols
    "×": "",
    "÷": "",
    "±": "",
}

# Reddit-specific allowed characters (layouts.wiki standard)
REDDIT_ALLOWED_CHARS = set("abcdefghijklmnopqrstuvwxyz .,'\"?-/:;\n")

# Accented characters to ASCII (for English or non-accented languages)
ACCENT_TO_ASCII = {
    "Á": "A",
    "Ã": "A",
    "Å": "A",
    "Æ": "AE",
    "Ç": "C",
    "É": "E",
    "È": "E",
    "Ê": "E",
    "Ë": "E",
    "Í": "I",
    "Ì": "I",
    "Î": "I",
    "Ï": "I",
    "Ñ": "N",
    "Ó": "O",
    "Ò": "O",
    "Ô": "O",
    "Õ": "O",
    "Ø": "O",
    "Œ": "OE",
    "Ú": "U",
    "Ù": "U",
    "Û": "U",
    "Ü": "U",
    "Ý": "Y",
    "à": "a",
    "á": "a",
    "â": "a",
    "ã": "a",
    "å": "a",
    "æ": "ae",
    "ç": "c",
    "è": "e",
    "é": "e",
    "ê": "e",
    "ë": "e",
    "ì": "i",
    "í": "i",
    "î": "i",
    "ï": "i",
    "ð": "d",
    "ñ": "n",
    "ò": "o",
    "ó": "o",
    "ô": "o",
    "õ": "o",
    "ø": "o",
    "œ": "oe",
    "ß": "ss",
    "ú": "u",
    "û": "u",
    "ü": "u",
    "ý": "y",
    "ÿ": "y",
    "ā": "a",
    "ă": "a",
    "ć": "c",
    "č": "c",
    "ī": "i",
    "ł": "l",
    "ŋ": "ng",
    "ō": "o",
    "Š": "S",
    "š": "s",
    "ū": "u",
    "Ž": "Z",
    "ž": "z",
}

# Non-French accents that should be converted even in French mode
NON_FRENCH_ACCENTS = {
    "ß": "ss",
    "Å": "A",
    "å": "a",
    "Ø": "O",
    "ø": "o",
    "ð": "d",
    "þ": "th",
    "Þ": "TH",
    "ñ": "n",
    "Ñ": "N",
    "ã": "a",
    "õ": "o",
}


def get_allowed_characters(language: Language = "english") -> set[str]:
    """Get the set of allowed characters for a given language."""
    if language == "french":
        return set(TYPABLE_CHARS + FRENCH_CHARS)
    else:
        return set(TYPABLE_CHARS)


def get_replacements(language: Language = "english") -> dict[str, str]:
    """Get character replacements for a given language."""
    replacements = BASE_REPLACEMENTS.copy()

    if language == "french":
        # For French: only convert non-French accents
        # French accents (é, è, à, etc.) are preserved
        replacements.update(NON_FRENCH_ACCENTS)
    else:
        # For English: convert accents to ASCII (café → cafe)
        # This preserves valid bigrams/trigrams instead of creating invalid ones
        # Example: "café" → "cafe" gives bigrams "ca","af","fe" (valid)
        #          vs removing "é" → "caf" gives "ca","af","f?" (invalid)
        replacements.update(ACCENT_TO_ASCII)

    return replacements


def apply_replacements(text: str, replacements: dict[str, str]) -> str:
    """Apply character replacements to text."""
    for old_char, new_char in replacements.items():
        text = text.replace(old_char, new_char)
    return text


def filter_characters(text: str, allowed_chars: set[str]) -> str:
    """Keep only allowed characters."""
    return "".join(char for char in text if char in allowed_chars)


def process_chunk(
    chunk: str, replacements: dict[str, str], allowed_chars: set[str]
) -> str:
    """Process a chunk of text: apply replacements and filter characters."""
    chunk = apply_replacements(chunk, replacements)
    chunk = filter_characters(chunk, allowed_chars)
    return chunk


def merge_lines(text: str, keep_ratio: float = 0.2) -> str:
    """Merge lines by replacing some newlines with spaces."""
    keep_every_n = int(1 / keep_ratio)
    return re.sub(
        "(\n)",
        lambda m, c=itertools.count(): m.group()
        if next(c) % keep_every_n == (keep_every_n - 1)
        else " ",
        text,
    )


@contextmanager
def temp_file_cleanup(path: Path) -> Iterator[Path]:
    """Context manager to ensure temporary file cleanup."""
    try:
        yield path
    finally:
        if path.exists():
            path.unlink()


def clean_corpus(
    input_path: str | Path,
    output_path: str | Path,
    language: Language = "english",
    remove_line_numbers: bool = False,
    merge_lines_ratio: float = DEFAULT_MERGE_LINES_RATIO,
    chunk_size: int = DEFAULT_CHUNK_SIZE,
) -> None:
    """Clean a corpus file with filtering and normalization.

    Args:
        input_path: Path to input corpus file
        output_path: Path to output cleaned file
        language: "english" or "french"
        remove_line_numbers: If True, remove leading line numbers (Leipzig format)
        merge_lines_ratio: Ratio of newlines to keep (0.2 = 1 out of 5)
        chunk_size: Size of chunks to process (default 50MB)
    """
    input_path = Path(input_path)
    output_path = Path(output_path)

    allowed_chars = get_allowed_characters(language)
    replacements = get_replacements(language)
    temp_file = output_path.with_suffix(output_path.suffix + ".tmp")
    intermediate_file = output_path.with_suffix(output_path.suffix + ".intermediate")

    with temp_file_cleanup(temp_file), temp_file_cleanup(intermediate_file):
        if remove_line_numbers:
            typer.echo("Step 1: Removing line numbers...")
            with temp_file.open("w") as f:
                subprocess.run(
                    ["cut", "-f2", str(input_path)],
                    stdout=f,
                    check=True,
                )
            process_input = temp_file
            step_offset = 1
        else:
            process_input = input_path
            step_offset = 0

        typer.echo(
            f"Step {1 + step_offset}: Applying character replacements and filtering..."
        )
        with (
            process_input.open("r", encoding="utf-8") as infile,
            intermediate_file.open("w", encoding="utf-8") as outfile,
        ):
            while True:
                chunk = infile.read(chunk_size)
                if not chunk:
                    break
                processed = process_chunk(chunk, replacements, allowed_chars)
                outfile.write(processed)

        if merge_lines_ratio > 0:
            typer.echo(
                f"Step {2 + step_offset}: Merging lines ({int(merge_lines_ratio * 100)}% newlines kept)..."
            )
            text = intermediate_file.read_text(encoding="utf-8")
            text = merge_lines(text, merge_lines_ratio)
            output_path.write_text(text, encoding="utf-8")
        else:
            intermediate_file.rename(output_path)

    typer.echo(f"✓ Cleaned corpus written to {output_path}")


app = typer.Typer(help="Clean text corpora for keyboard layout optimization")


@app.command()
def main(
    input_path: Annotated[
        Path,
        typer.Argument(
            help="Path to input corpus file",
            exists=True,
            file_okay=True,
            dir_okay=False,
            readable=True,
        ),
    ],
    output_path: Annotated[Path, typer.Argument(help="Path to output cleaned file")],
    language: Annotated[
        Language, typer.Option(help="Language for character filtering")
    ] = "english",
    remove_line_numbers: Annotated[
        bool,
        typer.Option(
            "--remove-line-numbers", help="Remove leading line numbers (Leipzig format)"
        ),
    ] = False,
    merge_lines_ratio: Annotated[
        float, typer.Option(help="Ratio of newlines to keep (0.2 = 1 out of 5)")
    ] = 0.2,
    chunk_size: Annotated[
        int, typer.Option(help="Size of chunks to process in bytes")
    ] = DEFAULT_CHUNK_SIZE,
) -> None:
    """Clean a corpus file with filtering and normalization.

    Supports English and French language cleaning with appropriate
    character sets and replacements for each language.
    """
    clean_corpus(
        input_path,
        output_path,
        language=language,
        remove_line_numbers=remove_line_numbers,
        merge_lines_ratio=merge_lines_ratio,
        chunk_size=chunk_size,
    )


if __name__ == "__main__":
    app()
