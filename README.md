# Svalboard Layout Optimizer

A keyboard layout optimizer forked from [catvw/keyboard_layout_optimizer](https://github.com/catvw/keyboard_layout_optimizer) which added Svalboard support to the original [dariogoetz/keyboard_layout_optimizer](https://github.com/dariogoetz/keyboard_layout_optimizer). This project enhances the optimizer with streamlined workflows, easier layout comparison through CSV and markdown tables, and basic French language support.

## Features

- **Layout Evaluation**: Analyze typing efficiency using various metrics (finger balance, key costs, bigrams, trigrams, cost-based scissors, SFB, etc.)
- **Layout Optimization**: Generate optimal layouts using genetic algorithms or simulated annealing
- **Multi-language Support**: Enhanced n-gram datasets for English, French, and bilingual optimization
- **Svalboard Support**: Built-in support for the [Svalboard](https://svalboard.com/products/lightly) keyboard with custom metrics
- **Streamlined Workflow**: Task automation using Taskfile
- **Flexible Configuration**: Highly customizable metrics and optimization parameters

## Installation

### Prerequisites

Install the required tools:

- **Rust**: Follow the installation guide at [rustup.rs](https://rustup.rs/)
- **Taskfile** (task runner): See installation instructions at [taskfile.dev/installation](https://taskfile.dev/installation/)
- **uv** (Python package manager, for result processing): See installation guide at [docs.astral.sh/uv/getting-started/installation](https://docs.astral.sh/uv/getting-started/installation/)

### Build the Project

```bash
# Clone the repository
git clone https://github.com/jeffzi/svalboard_layout_optimizer
cd svalboard_layout_optimizer

# Build the project
cargo build --release
```

## Quick Start

**Important**: All commands should be run from the project root directory, not from subdirectories like `ngrams/`.

The project uses [Taskfile](https://taskfile.dev/) to streamline common operations. Taskfile wraps the base CLI commands (see [Advanced Usage](#advanced-usage)) and makes it easier to:

- Manage input/output files with sensible defaults
- Evaluate multiple layouts concurrently
- Generate comprehensive reports (CSV, markdown, SVG) automatically

The main workflows are `optimize` and `evaluate`.

### First Time Setup

Before optimizing, you need a file containing starting layouts (one per line). You can:

1. **Start with a known layout** like QWERTY:

   ```bash
   # Create a starting layouts file
   echo "q□a□zw□sbxe□dtcr□fgvuhj'miyk□,onl□.p-?□□" > eng_shai_layouts.txt
   ```

2. **Use an existing optimized layout** from the community as a starting point

### Complete Workflow Example

```bash
# 1. Create a starting layouts file (replace with your preferred layout)
echo "q□a□zw□sbxe□dtcr□fgvuhj'miyk□,onl□.p-?□□" > eng_shai_layouts.txt

# 2. Run optimization (this will create eng_shai_optimized_layouts.txt)
task optimize CORPUS=eng_shai

# 3. Results are automatically generated in evaluation/eng_shai/
ls evaluation/eng_shai/
```

### Optimize Layouts

Generate optimized layouts for a specific language corpus (must be in [ngrams/](ngrams/)).

**Prerequisites**: You need an input layouts file containing starting layouts (one per line). By default, the task looks for `<CORPUS>_layouts.txt` (e.g., `eng_shai_layouts.txt`).

```bash
# Optimize for English corpus (requires eng_shai_layouts.txt)
task optimize CORPUS=eng_shai

# Use a custom input file
task optimize CORPUS=eng_shai IN_LAYOUT_FILE=my_starts.txt

# Optimize with custom parameters (fix certain keys)
task optimize CORPUS=eng_fra -- --fix 'reoyaui'

# See optimization options
task optimize CORPUS=eng_fra -- --help
```

The optimized layouts will be saved to `<CORPUS>_optimized_layouts.txt` and automatically evaluated.

### Evaluate Existing Layouts

Evaluate a file of layouts that were previously optimized:

```bash
# Evaluate previously optimized layouts
task evaluate CORPUS=eng_fra

# Evaluate a specific layout file
task evaluate CORPUS=eng_fra LAYOUT_FILE=my_layouts.txt
```

## Output

The `evaluate` task generates comprehensive results in the `evaluation/<corpus>/` directory:

- **CSV file**: Tabulated metrics for easy comparison
- **Markdown report**: Detailed analysis with layout visualizations
- **SVG diagrams**: Visual representations of each layout

The output is processed by [`scripts/parse_results.py`](scripts/parse_results.py) which enhances the raw evaluation data with frequency information and creates user-friendly summaries.

## Language Corpora

The project includes several n-gram datasets in the [`ngrams/`](ngrams/) directory:

### English

- `eng_shai`: **[Recommended]** [Shai's Cleaned iweb corpus](https://colemak.com/pub/corpus/iweb-corpus-samples-cleaned.txt.xz) (90M words) - A well-balanced English corpus. Named after Shai Coleman (Colemak creator) who cleaned and published this corpus.
- `eng_web_1m`, `eng_wiki_1m`: Web and Wikipedia corpora

### French

- `fra_news`, `fra_web`, `fra_wikipedia`: Individual French [Leipzig](https://wortschatz.uni-leipzig.de) corpora
- `fra_leipzig`: Combined Leipzig corpora with weighted ratios (web:50, news:30, wikipedia:20)

### Bilingual

- `eng_fra`: English-French bilingual corpus (eng_shai:70, fra_web:30)

All French ngrams were generated using [`scripts/corpora/Taskfile.yml`](scripts/corpora/Taskfile.yml).

## Configuration

### Evaluation Metrics

The main metrics configuration is in [`config/evaluation/sval.yml`](config/evaluation/sval.yml). Key metrics include:

- **finger_balance**: Ensures optimal finger load distribution based on intended loads per finger
- **hand_disbalance**: Maintains left-right hand balance
- **key_costs**: Penalizes hard-to-reach keys based on position difficulty
- **character_constraints**: Applies penalties when specific characters appear at specific positions. Configured here to restrict high-frequency double letters to comfortable positions (center/south)
- **sfb**: Same Finger Bigram metric that evaluates same-finger bigram comfort with directional costs
- **scissors**: Cost-based scissoring metric that penalizes adjacent finger movements with effort imbalances
- **manual_bigram_penalty**: Penalizes specific uncomfortable bigrams (e.g., pinky same-key repeats)
- **bigram_stats**: Provides statistics on bigram categories like SFB, scissor types, and other movement patterns (informational, weight: 0)
- **trigram_stats**: Tracks roll and redirect statistics (informational, weight: 0)

### Key Costs

Physical key costs are defined in [`config/keyboard/sval.yml`](config/keyboard/sval.yml) under the `key_costs` section. The Svalboard configuration reflects the dual homerow design where:

- **Center & South keys**: Most comfortable
- **Inward keys**: Moderately comfortable
- **Outward keys**: Less comfortable
- **North keys**: Least comfortable

### Svalboard-Specific Metrics

The optimizer includes custom metrics optimized for the Svalboard's unique geometry:

- **sfb**: Same Finger Bigram metric with directional costs:

  - Center→South movements are rewarded
  - Other directions penalized based on comfort
  - Finger multipliers increase penalties for weaker fingers
  - High-frequency SFBs get additional penalty multiplier

- **scissors**: Key-cost-based scissoring that identifies when adjacent fingers have mismatched effort (e.g., weak finger doing hard work while strong finger gets easy work). Uses the key costs defined in the keyboard configuration to calculate effort imbalances. Penalties scale proportionally with the absolute cost difference between keys and distinguish between movement types:

  - **Full Scissor Vertical**: Opposite vertical directions (North ↔ South)
  - **Full Scissor Squeeze**: Fingers moving toward each other (In ↔ Out, inward motion)
  - **Full Scissor Splay**: Fingers moving apart (In ↔ Out, outward motion)
  - **Half Scissor Diagonal**: Lateral + Vertical - One finger moves laterally (In/Out), other vertically (North/South)
  - **Half Scissor Lateral**: Lateral + Center - One finger moves laterally (In/Out), other presses Center
  - High-frequency scissors get additional penalty multiplier

- **position_penalties**: Penalizes specific characters at specific matrix positions. Currently configured to:
  - Restrict common double letters (e, l, s, o, t, r, h, n, f, p) to comfortable positions (center/south preferred)
  - Keep punctuation marks (,.'- ) off center keys to preserve homerow flow
  - This metric is highly customizable for enforcing character placement constraints

## Project Structure

```
├── config/
│   ├── evaluation/sval.yml    # Metrics configuration
│   └── keyboard/sval.yml      # Svalboard physical layout
├── ngrams/                    # Language corpora
├── scripts/
│   ├── parse_results.py       # Result processing
│   └── french/Taskfile.yml    # French corpus generation
├── evaluation/                # Generated evaluation results
└── Taskfile.yml              # Main task definitions
```

## Optimization Philosophy

The chosen metric weights aim to produce balanced layouts that:

1. **Respect hand/finger anatomy**: Strong fingers handle more load, weak fingers less
2. **Leverage Svalboard geometry**: Optimize for dual homerows and comfortable key positions
3. **Minimize discomfort**: Cost-based penalties for scissors (effort imbalances between adjacent fingers) and uncomfortable same-finger bigrams
4. **Reward natural motions**: Center→South movements and smooth finger transitions
5. **Balance typing flow**: Maintain good hand alternation while allowing efficient same-hand patterns

## Advanced Usage

### Direct Binary Usage

For more control or integration into custom workflows, you can use the compiled binaries directly instead of Taskfile:

```bash
# Evaluate a specific layout
cargo run --bin evaluate -- \
  --layout-config config/keyboard/sval.yml \
  --ngrams ngrams/eng_shai \
  "your layout string here"

# Optimize from a starting layout
cargo run --bin optimize_sa -- \
  --layout-config config/keyboard/sval.yml \
  --ngrams ngrams/eng_shai \
  --start-layouts "starting layout" \
  --append-solutions-to results.txt
```

### Layout String Format

Layouts are specified as space-separated strings representing keys from left to right, top to bottom. Use `□` for placeholder/empty positions:

```
□□gwc□□y□i□□o□u□□e□avxlmh□qnjt□zd□s□bfkpr
```

## Contributing

Contributions are welcome! Areas of particular interest:

- Additional language corpora
- Metric improvements and calibration

## License

This project inherits the GPL-3.0 license from the original keyboard_layout_optimizer.

## Troubleshooting

Make sure you're in the project root directory (where `Taskfile.yml` is located), not in subdirectories like `ngrams/`.

### "Error: Input layouts file '...\_layouts.txt' not found"

This means you need to create a starting layouts file before running optimization.

The default input filename follows the pattern: `<CORPUS>_layouts.txt`

- For `CORPUS=eng_shai`, it expects `eng_shai_layouts.txt`
- For `CORPUS=eng_fra`, it expects `eng_fra_layouts.txt`

**Solution**:

```bash
# Create a layouts file with a starting layout (filename must match corpus name)
echo "'□cqb-□i□y□?e□o□.a,um□hklgjt□dwxn□pvzs□fr" > eng_shai_layouts.txt

# Or specify a different file
task optimize CORPUS=eng_shai IN_LAYOUT_FILE=my_layouts.txt
```

### Optimization produces poor results

- **Check your corpus**: Make sure the ngram files match your target language
- **Adjust starting layouts**: Try different starting points or multiple starting layouts
- **Review metrics**: The weights in `config/evaluation/sval.yml` can be adjusted for your preferences
- **Fix important keys**: Use `-- --fix 'keys'` to keep certain letters in place

## Acknowledgments

- [dariogoetz](https://github.com/dariogoetz/keyboard_layout_optimizer) - Original optimizer framework
- [marcusbuffett](https://github.com/marcusbuffett/keyboard_layout_optimizer) - Svalboard metrics inspiration and [optimization insights](https://mbuffett.com/posts/optimizing-datahand-layout/)
- [catvw](https://github.com/catvw/keyboard_layout_optimizer) - Svalboard support and custom metrics implementation
- [Svalboard](https://svalboard.com/products/lightly) - The innovative keyboard this optimizer targets
