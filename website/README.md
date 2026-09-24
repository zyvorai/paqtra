# Paqtra docs site

Built with [Docusaurus](https://docusaurus.io/). Serves the live docs at https://zyvorai.github.io/paqtra/.

## Local development

```bash
npm install
npm start
```

## Build

```bash
npm run build
npm run serve   # preview the production build locally
```

## Images

Social cards are **not** duplicated into `website/static/`. `docusaurus.config.ts` serves `../docs/social` in place, so the README and this site reference the same files. Rebuild the cards with `docs/social/build-social-card.sh`.

## Docs content

The pages under `docs/` are adapted copies of the repo's flat product docs: `QUICKSTART.md` and `../docs/{architecture,cilium-brotherhood,ebpf-integration,features}.md`. They have the leading H1 replaced by front matter, and relative links rewritten to GitHub URLs. When you change one, change the other.

## Deployment

Deployment is automatic: `.github/workflows/pages.yml` builds and publishes this site to GitHub Pages on every push to `main` that touches `website/`, `docs/ux/`, or `docs/social/`. Pages must be enabled once with Source = "GitHub Actions". Don't use Docusaurus's built-in `deploy` script; it targets a `gh-pages` branch this repo doesn't use.
