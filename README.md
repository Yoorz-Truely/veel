# Veel

Veel is the project language of Annilay, a video editor built on Kdenlive's source. An Annilay project is a folder of `.veel` files that describe everything a `.kdenlive` file would hold: sources, bins, sequences, tracks, clips, effects and keyframes, compositions, captions and delivery. People and AI edit the files directly, and Annilay shows every change live and writes its own edits back.

Nothing is built yet. The specification is [docs/spec/veel-v0.md](docs/spec/veel-v0.md), a draft until phase 1 freezes it. Where Veel fits in the product is in Annilay's `docs/ROADMAP.md` (`../Annilay`), and the plan for this repository in Annilay's `docs/REPOSITORIES.md`. Veel writes Scowr's node syntax, through Scowr's front-end crates (`../Scowr`).

Veel is MIT-licensed ([LICENSE](LICENSE)). Annilay itself carries Kdenlive's GPL.

```sh
just check
```
