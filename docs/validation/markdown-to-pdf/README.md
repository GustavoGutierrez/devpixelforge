# Markdown-to-PDF CLI Validation

This directory contains a reproducible CLI validation for the `markdown_to_pdf` operation.

## Theme Choice

The validation uses GlyphWeaveForge's built-in `engineering` theme because both repository inputs are technical project documents.

## Validation Inputs

- `README.md`
- `AGENTS.md`

## Generated Artifacts

- `readme.pdf`
- `agents.pdf`

## Reproduce the Validation

Run these commands from the repository root:

```bash
mkdir -p docs/validation/markdown-to-pdf

./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"docs/validation/markdown-to-pdf/readme.pdf","theme":"engineering"}'

./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"AGENTS.md","output":"docs/validation/markdown-to-pdf/agents.pdf","theme":"engineering"}'
```

## Expected Result

- Each command returns a successful `markdown_to_pdf` job result.
- The generated PDFs begin with the `%PDF` signature.
- Relative assets referenced by the Markdown source remain resolvable because the conversion uses file-based input.

## Notes

- The output path is committed under `docs/validation/markdown-to-pdf/` so the validation artifacts live next to their reproduction guide.
- If you rebuild the binary, rerun the same commands to refresh the PDFs.
