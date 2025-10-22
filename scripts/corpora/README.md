# Corpus Processing Pipelines

Generate n-gram frequency tables for keyboard layout optimization from English (Reddit) and French (Leipzig) corpora.

## Quick Start

```bash
cd scripts/corpora
task  # Build all ngrams
```

**Prerequisites**: [Task](https://taskfile.dev/), [uv](https://github.com/astral-sh/uv), Rust toolchain

## Pipelines

| Task | Description | Output |
|------|-------------|--------|
| `task reddit` | English corpus from [ConvoKit Reddit (Small)](https://convokit.cornell.edu/documentation/reddit-small.html) | `ngrams/eng_reddit_small/` |
| `task french` | French corpora from [Leipzig](https://wortschatz.uni-leipzig.de/) (news, web, wikipedia) merged with weights 30%, 50%, 20% | `ngrams/fra_leipzig/` |
| `task merge-eng-fra` | Bilingual ngrams (80% English, 20% French) | `ngrams/eng_fra/` |

Each pipeline: **Download** → **Clean** → **Generate n-grams** → **Normalize**

## Text Preprocessing

All corpora use **consistent normalization rules** to produce comparable n-gram statistics.

### Shared Normalization (All Languages)

1. **Character normalization** - Smart quotes/dashes → ASCII, em-dash `—` → `-` (single)
2. **Symbol deletion** - Remove `™®©` and math symbols `×÷±` (rare in natural text)
3. **Punctuation simplification** - Ellipsis `…` → `.`, delete bullets `•·`

**Philosophy**: No character count inflation. Convert only what's typeable, delete what's not.

### Language-Specific Rules

**English (Reddit)**
- Lowercase conversion for case-insensitive statistics
- Character filtering: `a-z .,'\"?-/:;`
- Line merging: 80% newlines → spaces
- Whitespace normalization

**French (Leipzig)**
- Leipzig format handling (remove line numbers)
- Accent preservation: `é è à ç` etc.
- Script filtering: Remove Greek, Cyrillic
- Non-French accent conversion: `ß→ss, ø→o`

## Output Structure

```
ngrams/
├── eng_reddit_small/    # English Reddit corpus
├── fra_leipzig/         # Merged French (news 30%, web 50%, wiki 20%)
└── eng_fra/             # Bilingual (English 80%, French 20%)
    ├── 1-grams.txt      # Character frequencies
    ├── 2-grams.txt      # Bigram frequencies
    └── 3-grams.txt      # Trigram frequencies
```

## Maintenance

**Clean all generated files**: `task clean`

**Add new corpus**: Create script in `corpora_builder/`, add tasks to `Taskfile.yml`, update this README

## References

- [ConvoKit](https://convokit.cornell.edu/) - Reddit conversation corpus
- [Leipzig Corpora](https://wortschatz.uni-leipzig.de/) - Multilingual text collections
- [Layouts Wiki](https://layouts.wiki/reference/resources/frequencies/) - Keyboard layout frequency analysis
