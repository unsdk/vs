# vs documentation site

This directory contains the Fumadocs + Next.js documentation site for `vs`.

## Local development

```bash
pnpm install
pnpm dev
```

## Validation

```bash
pnpm types:check
pnpm build
```

## Deployment

GitHub Actions builds the static export in `out/` and deploys it to GitHub Pages for `https://vs.nn.ci`.
