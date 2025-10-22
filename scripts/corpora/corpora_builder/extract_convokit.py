#!/usr/bin/env python3
"""Extract text from ConvoKit Reddit corpus to a plain text file."""

from pathlib import Path

import typer
from typing_extensions import Annotated
from convokit import Corpus, download

app = typer.Typer(help="Extract text from ConvoKit Reddit corpus")


def extract_reddit_corpus(output_file: Path) -> None:
    """Download and extract text from Reddit corpus."""
    typer.echo("Downloading reddit-corpus-small...")
    corpus = Corpus(filename=download("reddit-corpus-small"))

    typer.echo(f"Processing {len(corpus.utterances)} utterances...")

    with open(output_file, "w", encoding="utf-8") as f:
        for utterance in corpus.iter_utterances():
            text = utterance.text
            if text and text.strip():
                cleaned_text = " ".join(text.split())
                f.write(cleaned_text + "\n")

    typer.echo(f"✓ Extracted text to {output_file}")


@app.command()
def main(
    outfile: Annotated[Path, typer.Argument(help="Output text file path")],
) -> None:
    """Extract text from ConvoKit Reddit corpus to a plain text file.

    Downloads the reddit-corpus-small dataset and extracts all utterances
    to a text file, one per line with cleaned whitespace.
    """
    extract_reddit_corpus(outfile)


if __name__ == "__main__":
    app()
