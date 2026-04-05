# Markdown-to-PDF CLI Validation

This directory contains a reproducible CLI validation for the `markdown_to_pdf` operation.

## Theme Coverage

The validation keeps the repository-document examples in the built-in `engineering` theme and also generates one PDF for each built-in GlyphWeaveForge theme.

## Validation Inputs

- `README.md`
- `AGENTS.md`

## Generated Artifacts

- `readme.pdf`
- `agents.pdf`
- `themes/invoice.pdf`
- `themes/scientific-article.pdf`
- `themes/professional.pdf`
- `themes/engineering.pdf`
- `themes/informational.pdf`

## Reproduce the Validation

Run these commands from the repository root:

```bash
mkdir -p docs/validation/markdown-to-pdf
mkdir -p docs/validation/markdown-to-pdf/themes

./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"docs/validation/markdown-to-pdf/readme.pdf","theme":"engineering"}'

./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"AGENTS.md","output":"docs/validation/markdown-to-pdf/agents.pdf","theme":"engineering"}'

for theme in invoice scientific_article professional engineering informational; do \
  ./dpf/target/release/dpf process \
    --job "{\"operation\":\"markdown_to_pdf\",\"input\":\"dpf/test_fixtures/sample.md\",\"output\":\"docs/validation/markdown-to-pdf/themes/${theme//_/-}.pdf\",\"theme\":\"${theme}\"}"; \
done
```

## Expected Result

- Each command returns a successful `markdown_to_pdf` job result.
- The generated PDFs begin with the `%PDF` signature.
- Each built-in theme generates a non-blank PDF artifact.
- Relative assets referenced by the Markdown source remain resolvable because the conversion uses file-based input.

## Notes

- The output path is committed under `docs/validation/markdown-to-pdf/` so the validation artifacts live next to their reproduction guide.
- If you rebuild the binary, rerun the same commands to refresh the PDFs.
- The current `dpf` build resolves `glyphweaveforge 0.1.3` directly from crates.io, so this validation reflects the published dependency path.
